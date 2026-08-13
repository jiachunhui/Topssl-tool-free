//! 系统类 command：应用信息 / 平台信息 / 端口探测 / 打开路径 / 剪贴板

use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub platform: String,
    pub arch: String,
    pub http01_privilege_note: Option<String>,
    pub certs_dir_template: String,
}

#[tauri::command]
pub fn get_app_info() -> AppResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        name: "SSL 证书助手".to_string(),
    })
}

#[tauri::command]
pub fn get_platform_info(state: tauri::State<'_, AppState>) -> AppResult<PlatformInfo> {
    Ok(PlatformInfo {
        platform: state.platform.clone(),
        arch: std::env::consts::ARCH.to_string(),
        http01_privilege_note: crate::util::port::http01_privilege_note(&state.platform),
        certs_dir_template: state.certs_root.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn probe_port80(state: tauri::State<'_, AppState>) -> AppResult<crate::util::port::PortStatus> {
    let port = crate::storage::settings::get_i64(&state.db.lock(), "http01_port", 80).clamp(1, 65535) as u16;
    Ok(crate::util::port::probe_port(port))
}

/// 打开路径（系统资源管理器 / 默认应用）
#[tauri::command]
pub fn open_path(path: String, app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| crate::error::AppError::internal(e))?;
    Ok(())
}

#[tauri::command]
pub fn copy_to_clipboard(text: String, app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| crate::error::AppError::internal(e))?;
    Ok(())
}

/// 获取应用内日志（最新的在前）
#[tauri::command]
pub fn get_logs(limit: Option<usize>) -> AppResult<Vec<crate::logs::LogEntry>> {
    Ok(crate::logs::get(limit.unwrap_or(300)))
}

/// 清空应用内日志
#[tauri::command]
pub fn clear_logs() -> AppResult<()> {
    crate::logs::clear();
    Ok(())
}

/// 前端把运行错误写入应用日志（排查问题用）
#[tauri::command]
pub fn frontend_log(level: String, msg: String) -> AppResult<()> {
    match level.as_str() {
        "error" => log::error!("[frontend] {msg}"),
        "warn" => log::warn!("[frontend] {msg}"),
        _ => log::info!("[frontend] {msg}"),
    }
    Ok(())
}
