//! IIS 自动部署（仅 Windows）
//!
//! 能力：检测 IIS 是否安装、是否管理员、列出站点；
//! 部署：把证书导入本机证书库（LocalMachine\My），并为指定站点添加
//! https 绑定（443 端口）与证书关联。
//! 需要管理员权限：应用内通过 is_elevated 检测，未提权时返回明确提示。

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

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
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("无法执行 {program}：{e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !out.status.success() {
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(stdout)
}

fn run_powershell(script: &str) -> Result<String, String> {
    run("powershell", &["-NoProfile", "-NonInteractive", "-Command", script])
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
        return Err(AppError::new(
            ErrorCode::Deploy,
            "IIS 部署需要管理员权限，请右键以管理员身份运行 Tossl 后重试",
        ));
    }
    if site_name.trim().is_empty() {
        return Err(AppError::new(ErrorCode::Deploy, "请选择要部署的 IIS 站点"));
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
        return Err(AppError::new(
            ErrorCode::Deploy,
            "证书目录中缺少 cert.pfx，请先重新申请证书后再部署",
        ));
    }

    // 1) 导入证书到本机证书库（LocalMachine\My，管理员权限下写入）
    let import = format!(
        "Import-PfxCertificate -FilePath '{}' -CertStoreLocation Cert:\\LocalMachine\\My -Password (ConvertTo-SecureString '{}' -AsPlainText -Force) -Exportable | Out-Null",
        pfx.to_string_lossy().replace('\'', "''"),
        crate::cert::store::IIS_PFX_PASSWORD,
    );
    run_powershell(&import)
        .map_err(|e| AppError::new(ErrorCode::Deploy, format!("导入证书到本机证书库失败：{e}")))?;

    // 2) 取该域名最近签发的证书指纹（通配符域名 Subject 为 CN=*.example.com）
    let thumb_script = format!(
        "(Get-ChildItem Cert:\\LocalMachine\\My | Where-Object {{ $_.Subject -like '*.{}*' }} | Sort-Object NotAfter -Descending | Select-Object -First 1).Thumbprint",
        cert.domain,
    );
    let thumb = run_powershell(&thumb_script)
        .map_err(|e| AppError::new(ErrorCode::Deploy, format!("获取证书指纹失败：{e}")))?
        .trim()
        .to_string();
    if thumb.is_empty() {
        return Err(AppError::new(
            ErrorCode::Deploy,
            "未在证书库中找到该域名的证书，请确认证书已导入成功",
        ));
    }

    // 3) 添加 https 绑定（443 端口，主机名可留空 = 所有主机名）
    let host_part = if host.trim().is_empty() { "*".to_string() } else { host.trim().to_string() };
    let site_esc = site_name.replace('"', "\\\"");
    let bind = format!(
        "& \"{appcmd}\" set site \"{site}\" /+bindings.[protocol='https',bindingInformation='*:443:{host}']",
        appcmd = appcmd_path(),
        site = site_esc,
        host = host_part,
    );
    // 绑定已存在时 appcmd 会报错，忽略即可（幂等）
    let _ = run_powershell(&bind);

    // 4) 绑定证书到该 https 绑定
    let ssl = format!(
        "& \"{appcmd}\" set site \"{site}\" /+sslBindings.[certificateHash={thumb},certificateStoreName=My]",
        appcmd = appcmd_path(),
        site = site_esc,
    );
    run_powershell(&ssl)
        .map_err(|e| AppError::new(ErrorCode::Deploy, format!("为站点绑定证书失败：{e}")))?;

    log::info!(
        "iis deploy ok: cert={} domain={} site={} host={} thumb={thumb}",
        cert_id,
        cert.domain,
        site_name,
        host_part,
    );
    Ok(format!(
        "部署完成：已为 IIS 站点「{site}」添加 https 绑定（443 端口，主机名 {host}），并关联证书（指纹 {thumb}）。\n请确认该站点根目录内容可达，然后访问 https://{domain} 验证。",
        site = site_name,
        host = host_part,
        thumb = thumb,
        domain = cert.domain,
    ))
}
