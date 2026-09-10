//! 数据迁移：导出 / 导入加密备份包
//!
//! 导出写到系统下载目录（与 export_deploy_package 一致，后端自己算路径，不需要文件对话框）；
//! 导入由前端用 `<input type="file">` 读取备份文件内容后以 base64 传入——项目未依赖
//! tauri-plugin-dialog，这样无需新增原生依赖，也就不涉及能力（capability）变更。

use tauri::{Emitter, Manager};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

/// 导出加密备份包到系统下载目录，返回备份文件路径
#[tauri::command]
pub fn export_backup_package(
    password: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<String> {
    let download_dir = app.path().download_dir().map_err(|e| {
        AppError::new(ErrorCode::Backup, "无法定位系统下载目录").detail(e.to_string())
    })?;
    let path = crate::backup::export(&state, &password, &download_dir)?;
    Ok(path.to_string_lossy().into_owned())
}

/// 导入备份包（覆盖当前设置、证书与密钥；导入前会自动备份当前数据）
///
/// `data`：备份文件内容的 base64。前端读取文件字节后传入。
#[tauri::command]
pub fn import_backup_package(
    password: String,
    data: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<crate::backup::ImportSummary> {
    use base64::Engine;
    let blob = base64::engine::general_purpose::STANDARD
        .decode(data.as_bytes())
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法读取备份文件内容").detail(e.to_string()))?;
    let summary = crate::backup::import(&state, &password, &blob)?;
    // 证书列表已整体替换，通知前端刷新
    let _ = app.emit("certs://changed", ());
    Ok(summary)
}
