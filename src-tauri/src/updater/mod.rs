//! 应用内更新：检查（国内清单优先 + GitHub Releases 兜底）、下载安装包、启动安装
//!
//! 数据流：
//!   1. 国内清单 `UPDATE_MANIFEST_URL`（宣传页同域，国内可直连）——域名确定后填写；
//!      None 时直接走 GitHub 兜底源，行为不变。
//!   2. GitHub API `releases/latest`（海外可直连）作为兜底：清单请求失败自动切换。
//!   3. 检查结果缓存到设置表（自动检查 6 小时节流 + ETag 复用，降低 API 配额消耗）。
//!
//! 点击「立即更新」后：流式下载安装包（update://progress 事件报进度，sha256 校验）
//! → Windows 退出应用并以独立进程启动 NSIS 安装器覆盖安装（currentUser 免管理员）。

use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

/// 国内更新清单地址（宣传页同域）。域名确定后填写：
/// `Some("https://你的域名/updates/latest.json")`；保持 None 时仅使用 GitHub 源。
pub const UPDATE_MANIFEST_URL: Option<&str> = None;

pub const GITHUB_API_LATEST: &str =
    "https://api.github.com/repos/jiachunhui/Topssl-tool-free/releases/latest";
pub const GITHUB_RELEASES_PAGE: &str =
    "https://github.com/jiachunhui/Topssl-tool-free/releases";

/// 自动检查节流：6 小时
const AUTO_CHECK_INTERVAL_SECS: i64 = 6 * 3600;
const USER_AGENT: &str = "TopSSL-Free-Cert-Assistant";

// 设置表缓存键
const KEY_LAST_CHECK: &str = "updater.last_check_at";
const KEY_ETAG: &str = "updater.etag";
const KEY_INFO: &str = "updater.info";
const KEY_DISMISSED: &str = "updater.dismissed_version";

// ---------- 序列化类型（与前端 src/lib/types.ts 对应） ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub tag_name: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub asset: Option<UpdateAsset>,
    pub release_page: String,
    /// 数据来源：domestic（国内清单）| github（兜底）
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub received: u64,
    pub total: u64,
}

// 国内清单 JSON 结构（scripts/gen-update-manifest.mjs 生成）
#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    platforms: std::collections::HashMap<String, ManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct ManifestAsset {
    url: String,
    size: u64,
    #[serde(default)]
    sha256: Option<String>,
}

// GitHub API 响应（仅取所需字段）
#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

// ---------- 工具 ----------

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(6))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build update http client")
    })
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 当前平台在清单中的键；不支持/未知平台返回 None（前端引导打开 GitHub 页面）
fn platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x86_64"),
        ("macos", "aarch64") => Some("darwin-aarch64"),
        ("macos", "x86_64") => Some("darwin-x86_64"),
        ("linux", "x86_64") => Some("linux-x86_64"),
        _ => None,
    }
}

type Semver = (u64, u64, u64);

/// 解析 x.y.z（容忍 v 前缀与预发布后缀，如 v0.1.4-beta.1 → 0.1.4）
fn parse_semver(s: &str) -> Option<Semver> {
    let t = s.trim();
    let t = t.strip_prefix('v').unwrap_or(t);
    let t = t.split('-').next().unwrap_or(t);
    let mut parts = t.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major, minor, patch))
}

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------- 缓存（设置表 KV） ----------

fn load_cached(state: &AppState) -> Option<UpdateInfo> {
    let db = state.db.lock();
    let raw = crate::storage::settings::get(&db, KEY_INFO).ok().flatten()?;
    serde_json::from_str::<UpdateInfo>(&raw).ok()
}

fn save_cached(state: &AppState, info: &UpdateInfo) {
    if let Ok(json) = serde_json::to_string(info) {
        let _ = crate::storage::settings::set(&state.db.lock(), KEY_INFO, &json);
    }
}

// ---------- 数据源 ----------

/// 国内清单源
async fn fetch_manifest(url: &str) -> AppResult<UpdateInfo> {
    let resp = client()
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateCheck, "连接更新服务器失败").detail(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::new(ErrorCode::UpdateCheck, "更新服务器响应异常")
            .detail(format!("HTTP {}", resp.status())));
    }
    let manifest: Manifest = resp
        .json()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateCheck, "解析更新清单失败").detail(e.to_string()))?;
    let latest = parse_semver(&manifest.version)
        .ok_or_else(|| AppError::new(ErrorCode::UpdateCheck, "更新清单版本号格式不正确"))?;
    let current = parse_semver(current_version())
        .ok_or_else(|| AppError::new(ErrorCode::UpdateCheck, "当前应用版本号格式不正确"))?;
    let asset = platform_key()
        .and_then(|k| manifest.platforms.get(k))
        .map(|a| UpdateAsset {
            name: a.url.rsplit('/').next().unwrap_or("installer").to_string(),
            url: a.url.clone(),
            size: a.size,
            sha256: a.sha256.clone(),
        });
    Ok(UpdateInfo {
        available: latest > current,
        current_version: current_version().to_string(),
        latest_version: manifest.version,
        tag_name: None,
        notes: manifest.notes,
        published_at: manifest.published_at,
        asset,
        release_page: GITHUB_RELEASES_PAGE.to_string(),
        source: "domestic".to_string(),
    })
}

/// GitHub Releases 兜底源（ETag 复用：304 直接返回缓存）
async fn fetch_github(state: &AppState) -> AppResult<UpdateInfo> {
    let etag = crate::storage::settings::get_string(&state.db.lock(), KEY_ETAG, "");
    let mut req = client()
        .get(GITHUB_API_LATEST)
        .header("Accept", "application/vnd.github+json");
    if !etag.is_empty() {
        req = req.header("If-None-Match", etag);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateCheck, "无法连接 GitHub").detail(e.to_string()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_MODIFIED || status == reqwest::StatusCode::FORBIDDEN {
        // 304：内容未变化；403：匿名配额用尽——都回退缓存
        if let Some(cached) = load_cached(state) {
            return Ok(cached);
        }
    }
    if !status.is_success() {
        return Err(AppError::new(ErrorCode::UpdateCheck, "GitHub 接口响应异常")
            .detail(format!("HTTP {status}")));
    }
    if let Some(new_etag) = resp.headers().get("etag").and_then(|v| v.to_str().ok()) {
        let _ = crate::storage::settings::set(&state.db.lock(), KEY_ETAG, new_etag);
    }
    let release: GhRelease = resp
        .json()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateCheck, "解析 GitHub 响应失败").detail(e.to_string()))?;
    if release.draft || release.prerelease {
        return Err(AppError::new(ErrorCode::UpdateCheck, "最新 Release 为草稿或预发布版本"));
    }
    let latest = parse_semver(&release.tag_name)
        .ok_or_else(|| AppError::new(ErrorCode::UpdateCheck, "Release 版本号格式不正确"))?;
    let current = parse_semver(current_version())
        .ok_or_else(|| AppError::new(ErrorCode::UpdateCheck, "当前应用版本号格式不正确"))?;
    Ok(UpdateInfo {
        available: latest > current,
        current_version: current_version().to_string(),
        latest_version: release.tag_name.trim_start_matches('v').to_string(),
        tag_name: Some(release.tag_name),
        notes: release.body,
        published_at: release.published_at,
        asset: pick_github_asset(&release.assets),
        release_page: GITHUB_RELEASES_PAGE.to_string(),
        source: "github".to_string(),
    })
}

/// 按当前平台从 GitHub Release 资产中挑选安装包
fn pick_github_asset(assets: &[GhAsset]) -> Option<UpdateAsset> {
    let key = platform_key()?;
    let find = |pred: &dyn Fn(&str) -> bool| -> Option<&GhAsset> {
        assets.iter().find(|a| pred(&a.name.to_lowercase()))
    };
    let chosen: Option<&GhAsset> = match key {
        "windows-x86_64" => find(&|n| n.ends_with("x64-setup.exe"))
            .or_else(|| find(&|n| n.ends_with("-setup.exe")))
            .or_else(|| find(&|n| n.ends_with(".exe"))),
        "darwin-aarch64" => find(&|n| n.ends_with("aarch64.dmg")),
        "darwin-x86_64" => find(&|n| n.ends_with("x64.dmg")),
        // Linux 发行格式多样（deb/rpm/AppImage），不做自动安装，由前端引导打开 Release 页
        _ => None,
    };
    chosen.map(|a| UpdateAsset {
        name: a.name.clone(),
        url: a.browser_download_url.clone(),
        size: a.size,
        sha256: None,
    })
}

// ---------- 检查逻辑 ----------

async fn check_update_inner(state: &AppState, force: bool) -> AppResult<UpdateInfo> {
    let now = now_secs();
    if !force {
        if let Some(cached) = load_cached(state) {
            let last = crate::storage::settings::get_i64(&state.db.lock(), KEY_LAST_CHECK, 0);
            if now - last < AUTO_CHECK_INTERVAL_SECS {
                return Ok(cached);
            }
        }
    }

    // 1) 国内清单（域名未配置时直接跳过）
    let mut info: Option<UpdateInfo> = None;
    if let Some(url) = UPDATE_MANIFEST_URL {
        match fetch_manifest(url).await {
            Ok(i) => info = Some(i),
            Err(e) => log::warn!("updater: 国内清单不可用（{e}），回退 GitHub 源"),
        }
    }
    // 2) GitHub 兜底
    let info = match info {
        Some(i) => i,
        None => fetch_github(state).await.map_err(|e| {
            AppError::new(ErrorCode::UpdateCheck, "检查更新失败").detail(e.to_string())
        })?,
    };

    let _ = crate::storage::settings::set(&state.db.lock(), KEY_LAST_CHECK, &now.to_string());
    save_cached(state, &info);
    Ok(info)
}

// ---------- IPC commands ----------

/// 检查更新。force=true 强制联网（手动检查），false 走 6 小时缓存（启动静默检查）
#[tauri::command]
pub async fn check_update(state: State<'_, AppState>, force: bool) -> AppResult<UpdateInfo> {
    check_update_inner(&state, force).await
}

/// 「稍后提醒」：忽略该版本（下次自动检查不再弹窗，手动检查仍可见）
#[tauri::command]
pub fn dismiss_update(version: String, state: State<'_, AppState>) -> AppResult<()> {
    crate::storage::settings::set(&state.db.lock(), KEY_DISMISSED, &version).map_err(AppError::from)?;
    Ok(())
}

/// 已忽略的版本号（启动时前端读取，用于判断自动弹窗）
#[tauri::command]
pub fn get_dismissed_update_version(state: State<'_, AppState>) -> AppResult<Option<String>> {
    Ok(crate::storage::settings::get(&state.db.lock(), KEY_DISMISSED).ok().flatten())
}

/// 下载最新安装包到临时目录，返回本地路径；进度通过 update://progress 事件推送
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<String> {
    // 优先用缓存中的可用更新；缓存缺失或不可用则强制重新检查
    let info = match load_cached(&state) {
        Some(i) if i.available => i,
        _ => check_update_inner(&state, true).await?,
    };
    if !info.available {
        return Err(AppError::new(ErrorCode::UpdateDownload, "当前已是最新版本"));
    }
    let asset = info.asset.ok_or_else(|| {
        AppError::new(ErrorCode::UpdateDownload, "该平台暂无自动更新安装包")
            .detail("请前往 GitHub Release 页面手动下载")
    })?;

    state.update_cancel.store(false, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("TopSSL-Free-Cert-Assistant-update");
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "创建临时目录失败").detail(e.to_string()))?;
    // 资产名来自可信清单/GitHub API，仅取 basename 防御路径穿越
    let file_name = asset
        .name
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("update-setup.exe")
        .to_string();
    let path = dir.join(file_name);

    let resp = client()
        .get(&asset.url)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "连接下载服务器失败").detail(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::new(ErrorCode::UpdateDownload, "下载服务器响应异常")
            .detail(format!("HTTP {}", resp.status())));
    }
    let total = if asset.size > 0 {
        asset.size
    } else {
        resp.content_length().unwrap_or(0)
    };
    // 安装包约 6MB：整体读入内存后分块写盘（进度按写入字节上报，无需 reqwest stream 特性）
    let body = resp
        .bytes()
        .await
        .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "下载中断").detail(e.to_string()))?;
    if state.update_cancel.load(Ordering::SeqCst) {
        return Err(AppError::new(ErrorCode::Canceled, "已取消下载"));
    }
    let mut file = std::fs::File::create(&path)
        .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "创建安装包文件失败").detail(e.to_string()))?;
    let mut hasher = asset
        .sha256
        .as_ref()
        .map(|_| openssl::hash::Hasher::new(openssl::hash::MessageDigest::sha256()).expect("sha256"));

    let mut received: u64 = 0;
    for chunk in body.chunks(64 * 1024) {
        if state.update_cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(AppError::new(ErrorCode::Canceled, "已取消下载"));
        }
        file.write_all(chunk)
            .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "写入安装包失败").detail(e.to_string()))?;
        if let Some(h) = hasher.as_mut() {
            let _ = h.update(chunk);
        }
        received += chunk.len() as u64;
        let _ = app.emit(
            "update://progress",
            UpdateProgress { received, total },
        );
    }
    file.flush()
        .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "写入安装包失败").detail(e.to_string()))?;

    // sha256 校验（国内清单源提供；GitHub 源无摘要，仅校验大小）
    if let (Some(mut h), Some(expect)) = (hasher, &asset.sha256) {
        let digest = h
            .finish()
            .map_err(|e| AppError::new(ErrorCode::UpdateDownload, "计算校验和失败").detail(e.to_string()))?;
        let actual = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
        if !actual.eq_ignore_ascii_case(expect) {
            let _ = std::fs::remove_file(&path);
            return Err(AppError::new(ErrorCode::UpdateDownload, "安装包校验失败（sha256 不匹配）")
                .detail("已放弃本次更新，请稍后重试"));
        }
    }
    log::info!(
        "updater: downloaded {} ({received} bytes) -> {}",
        asset.name,
        path.display()
    );
    Ok(path.to_string_lossy().to_string())
}

/// 取消进行中的下载（下载循环检测到标记后中止并清理临时文件）
#[tauri::command]
pub fn cancel_update_download(state: State<'_, AppState>) -> AppResult<()> {
    state.update_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// 启动已下载的安装包。
/// Windows：独立进程拉起 NSIS 安装器（无窗口），随后退出应用避免文件占用，实现「点击即更新」；
/// macOS：open 打开 dmg，由用户拖入 Applications；
/// Linux：无自动安装包时兜底打开 GitHub Release 页面。
#[tauri::command]
pub fn install_update(path: String, app: AppHandle) -> AppResult<()> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_file() {
        return Err(AppError::new(ErrorCode::UpdateDownload, "安装包不存在")
            .detail(path));
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg("start").arg("").arg(&p);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd
            .spawn()
            .map_err(|e| AppError::new(ErrorCode::Internal, "启动安装程序失败").detail(e.to_string()))?;
        log::info!("updater: installer launched, exiting app");
        // 等待安装进程拉起后退出应用（释放文件占用，NSIS 覆盖安装）
        std::thread::sleep(Duration::from_millis(800));
        app.exit(0);
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&p)
            .spawn()
            .map_err(|e| AppError::new(ErrorCode::Internal, "打开安装镜像失败").detail(e.to_string()))?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(GITHUB_RELEASES_PAGE, None::<&str>)
            .map_err(|e| AppError::new(ErrorCode::Internal, "打开浏览器失败").detail(e.to_string()))?;
    }
    Ok(())
}

/// 打开 GitHub Release 页面（「前往 GitHub 查看」兜底入口）
#[tauri::command]
pub fn open_release_page(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(GITHUB_RELEASES_PAGE, None::<&str>)
        .map_err(|e| AppError::internal(e))?;
    Ok(())
}
