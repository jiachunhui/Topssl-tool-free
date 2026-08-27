//! IIS 自动部署（仅 Windows）
//!
//! 能力：检测 IIS 是否安装、是否管理员、列出站点；
//! 部署流程（与 CertGuard Agent 对齐，全部在本机实测验证）：
//! 1. 从 cert.pfx 本地计算证书指纹（SHA-1，与导入证书库后的指纹一致）
//! 2. certutil 导入证书到本机证书库（LocalMachine\My）
//! 3. appcmd 删除旧 https 绑定（幂等）→ 添加 https 绑定（443 + 域名主机名）
//! 4. appcmd 设置 sslFlags:1（SNI，多域名共用 443 必需）
//! 5. PowerShell WebAdministration 的 WebBinding.AddSslCertificate 绑定证书
//!    （appcmd 的 certificateHash 属性不受支持，这是唯一可靠方式）
//! 需要管理员权限：应用内通过 is_elevated 检测，未提权时返回明确提示。
//! 子进程均以 CREATE_NO_WINDOW 启动，避免部署时弹出控制台窗口。

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// 隐藏子进程的控制台窗口（Tauri 为 GUI 子系统，直接 spawn 控制台程序会弹窗）
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IisSite {
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IisStatus {
    /// 当前系统是否支持（仅 Windows）
    pub supported: bool,
    /// 是否检测到 IIS（appcmd.exe 存在）
    pub installed: bool,
    /// 当前进程是否管理员（IIS 部署必须）
    pub elevated: bool,
    /// 检测到的 IIS 站点列表
    pub sites: Vec<IisSite>,
}

fn appcmd_path() -> &'static str {
    r"C:\Windows\System32\inetsrv\appcmd.exe"
}

fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().map_err(|e| format!("无法执行 {program}：{e}"))?;
    let stdout = decode_output(&out.stdout);
    let stderr = decode_output(&out.stderr);
    if !out.status.success() {
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(stdout)
}

fn run_powershell(script: &str) -> Result<String, String> {
    run("powershell", &["-NoProfile", "-NonInteractive", "-Command", script])
}

/// 解码子进程输出：优先按 UTF-8 严格解码，失败则回退 GBK
/// （中文 Windows 下 appcmd/certutil/PowerShell 的 stdout 为 GBK 编码）
fn decode_output(bytes: &[u8]) -> String {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::GBK.decode(bytes).0.into_owned(),
    };
    text.trim().trim_start_matches('\u{feff}').to_string()
}

/// 从 PFX 文件本地计算证书指纹（SHA-1 大写 hex，与导入证书库后的指纹一致），
/// 无需依赖导入结果，避免 PowerShell 往返
fn pfx_thumbprint(pfx: &std::path::Path, password: &str) -> Result<String, String> {
    let der = std::fs::read(pfx).map_err(|e| format!("无法读取 cert.pfx：{e}"))?;
    let pkcs12 = openssl::pkcs12::Pkcs12::from_der(&der)
        .map_err(|e| format!("cert.pfx 解析失败：{e}"))?;
    let parsed = pkcs12
        .parse2(password)
        .map_err(|e| format!("cert.pfx 密码错误或文件已损坏：{e}"))?;
    let leaf = parsed.cert.ok_or_else(|| "cert.pfx 中未包含证书".to_string())?;
    let digest = leaf
        .digest(openssl::hash::MessageDigest::sha1())
        .map_err(|e| format!("计算证书指纹失败：{e}"))?;
    Ok(digest.iter().map(|b| format!("{b:02X}")).collect())
}

/// 当前进程是否为管理员（Windows 内置角色判断）
fn is_elevated() -> bool {
    run_powershell(
        "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
    )
    .map(|s| s.trim().eq_ignore_ascii_case("true"))
    .unwrap_or(false)
}

/// 解析 `appcmd list sites` 输出中的站点名（SITE "Default Web Site" (id:1,...)）
fn list_sites() -> Result<Vec<IisSite>, String> {
    let out = run(appcmd_path(), &["list", "sites"])?;
    let mut sites = Vec::new();
    for line in out.lines() {
        if let Some(start) = line.find("SITE \"") {
            let rest = &line[start + 6..];
            if let Some(end) = rest.find('"') {
                sites.push(IisSite { name: rest[..end].to_string() });
            }
        }
    }
    Ok(sites)
}

/// 查询 IIS 与权限状态（前端据此决定是否展示/允许 IIS 部署）
#[tauri::command]
pub fn iis_status() -> IisStatus {
    if !is_windows() {
        return IisStatus { supported: false, installed: false, elevated: false, sites: vec![] };
    }
    let installed = std::path::Path::new(appcmd_path()).exists();
    let elevated = is_elevated();
    let sites = if installed { list_sites().unwrap_or_default() } else { vec![] };
    IisStatus { supported: true, installed, elevated, sites }
}

/// 一键部署：导入证书到本机证书库 + 为指定站点添加 https 绑定与证书
#[tauri::command]
pub fn iis_deploy_cert(
    cert_id: i64,
    site_name: String,
    host: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<String> {
    if !is_windows() {
        return Err(AppError::new(ErrorCode::Deploy, "IIS 部署仅支持 Windows 系统"));
    }
    if !is_elevated() {
        return Err(AppError::new(ErrorCode::Deploy, "IIS 部署需要管理员权限")
            .detail("请右键以管理员身份运行 ToSSL 后重试"));
    }
    if site_name.trim().is_empty() {
        return Err(AppError::new(ErrorCode::Deploy, "未选择站点")
            .detail("请选择要部署的 IIS 站点"));
    }

    let conn = state.db.lock();
    let cert = crate::storage::certificates::get(&conn, cert_id)?
        .ok_or_else(|| AppError::new(ErrorCode::Db, "证书不存在"))?;
    drop(conn);

    // 证书目录中应有 cert.pfx（IIS 导入用，密码见 store 常量）
    let cert_dir = std::path::Path::new(&cert.cert_chain_path)
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::Deploy, "无法定位证书目录"))?;
    let pfx = cert_dir.join("cert.pfx");
    if !pfx.exists() {
        return Err(AppError::new(ErrorCode::Deploy, "证书目录中缺少 cert.pfx")
            .detail("请先重新申请证书后再部署"));
    }

    // 主机名：留空时使用证书域名（SNI 绑定，与 IIS 管理器「需要服务器名称指示」一致）
    let hostname = if host.trim().is_empty() {
        cert.domain.trim().to_string()
    } else {
        host.trim().to_string()
    };
    if hostname.is_empty() || hostname == "*" {
        return Err(AppError::new(ErrorCode::Deploy, "无法确定绑定主机名").detail(
            "请填写要绑定的域名（留空将使用证书域名）",
        ));
    }

    // 1) 从 cert.pfx 本地计算指纹（SHA-1，与导入证书库后的指纹一致）
    let thumb = pfx_thumbprint(&pfx, crate::cert::store::IIS_PFX_PASSWORD)
        .map_err(|e| AppError::new(ErrorCode::Deploy, "读取证书指纹失败").detail(e))?;

    // 2) 导入证书到本机证书库（certutil -f 覆盖导入，无需 PowerShell）
    let pfx_arg = pfx.to_string_lossy().to_string();
    run(
        "certutil",
        &["-f", "-p", crate::cert::store::IIS_PFX_PASSWORD, "-importpfx", "My", &pfx_arg],
    )
    .map_err(|e| {
        log::warn!("iis deploy: certutil import failed: {e}");
        AppError::new(ErrorCode::Deploy, "导入证书到本机证书库失败").detail(e)
    })?;

    // 3) 删除旧的同名 https 绑定（幂等；绑定不存在时报错可忽略）
    let rm = format!("/-bindings.[protocol='https',bindingInformation='*:443:{hostname}']");
    let _ = run(appcmd_path(), &["set", "site", &site_name, &rm]);

    // 4) 添加 https 绑定（443 端口 + 域名主机名）
    let add = format!("/+bindings.[protocol='https',bindingInformation='*:443:{hostname}']");
    run(appcmd_path(), &["set", "site", &site_name, &add]).map_err(|e| {
        log::warn!("iis deploy: add https binding failed: {e}");
        AppError::new(ErrorCode::Deploy, "添加 https 绑定失败")
            .detail(format!("站点「{site_name}」添加 443 绑定失败：{e}"))
    })?;

    // 5) 启用 SNI（同一 443 端口多域名证书必需；失败仅告警，不影响绑定证书）
    let sni = format!("/bindings.[protocol='https',bindingInformation='*:443:{hostname}'].sslFlags:1");
    if let Err(e) = run(appcmd_path(), &["set", "site", &site_name, &sni]) {
        log::warn!("iis deploy: set sslFlags(SNI) failed: {e}");
    }

    // 6) 通过 WebAdministration 把证书绑定到该 https 绑定
    //    （appcmd 不支持设置 certificateHash/certificateStoreName，PowerShell 是唯一可靠方式）
    let site_esc = site_name.replace('\'', "''");
    let host_esc = hostname.replace('\'', "''");
    let script = format!(
        "Import-Module WebAdministration; $b = Get-WebBinding -Name '{site_esc}' -Protocol 'https' -HostHeader '{host_esc}'; if ($b) {{ $b.AddSslCertificate('{thumb}', 'MY') }} else {{ throw 'HTTPS binding not found' }}"
    );
    run_powershell(&script).map_err(|e| {
        log::warn!("iis deploy: AddSslCertificate failed: {e}");
        AppError::new(ErrorCode::Deploy, "为站点绑定证书失败")
            .detail(format!("站点「{site_name}」绑定证书失败：{e}"))
    })?;

    log::info!(
        "iis deploy ok: cert={cert_id} domain={} site={site_name} host={hostname} thumb={thumb}",
        cert.domain,
    );
    Ok(format!(
        "部署完成：已为 IIS 站点「{site_name}」添加 https 绑定（443 端口，主机名 {hostname}），并关联证书（指纹 {thumb}）。\n请访问 https://{hostname} 验证。"
    ))
}
