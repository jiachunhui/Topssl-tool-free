//! 设置类 command

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// 注意：字段用蛇形命名序列化（与前端 types.ts 的 Settings 接口一一对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub acme_directory: String,
    pub contact_email: String,
    pub auto_renew: bool,
    pub run_at_login: bool,
    pub http01_port: i64,
    pub default_provider_id: Option<i64>,
    /// 证书密钥类型：rsa（兼容性最好）| ecc（P-384，更快更安全）
    pub cert_key_type: String,
    /// 系统通知：证书到期提醒
    pub notify_expiring: bool,
    /// 系统通知：续期成功
    pub notify_renew_success: bool,
    /// 系统通知：续期失败
    pub notify_renew_failed: bool,
}

const DEFAULT_KEYS: [(&str, &str); 10] = [
    ("acme_directory", "staging"),
    ("contact_email", ""),
    ("auto_renew", "true"),
    ("run_at_login", "true"),
    ("http01_port", "80"),
    ("default_provider_id", ""),
    ("cert_key_type", "rsa"),
    ("notify_expiring", "true"),
    ("notify_renew_success", "true"),
    ("notify_renew_failed", "true"),
];

fn load_settings(state: &AppState) -> Settings {
    let conn = state.db.lock();
    let get = |k: &str| crate::storage::settings::get(&conn, k).ok().flatten();
    // 初始化默认值
    for (k, v) in DEFAULT_KEYS {
        if get(k).is_none() {
            let _ = crate::storage::settings::set(&conn, k, v);
        }
    }
    Settings {
        acme_directory: get("acme_directory").unwrap_or_else(|| "staging".into()),
        contact_email: get("contact_email").unwrap_or_default(),
        auto_renew: get("auto_renew").and_then(|v| v.parse().ok()).unwrap_or(true),
        run_at_login: get("run_at_login").and_then(|v| v.parse().ok()).unwrap_or(true),
        http01_port: get("http01_port").and_then(|v| v.parse().ok()).unwrap_or(80),
        default_provider_id: get("default_provider_id")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0),
        cert_key_type: get("cert_key_type").unwrap_or_else(|| "rsa".into()),
        notify_expiring: get("notify_expiring").and_then(|v| v.parse().ok()).unwrap_or(true),
        notify_renew_success: get("notify_renew_success").and_then(|v| v.parse().ok()).unwrap_or(true),
        notify_renew_failed: get("notify_renew_failed").and_then(|v| v.parse().ok()).unwrap_or(true),
    }
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> AppResult<Settings> {
    let s = load_settings(&state);
    log::info!(
        "settings read: directory={} http01_port={} auto_renew={} run_at_login={}",
        s.acme_directory,
        s.http01_port,
        s.auto_renew,
        s.run_at_login
    );
    Ok(s)
}

#[tauri::command]
pub fn set_setting(
    key: String,
    value: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    // 关键设置项校验
    if key == "http01_port" {
        let port: i64 = value
            .parse()
            .map_err(|_| AppError::new(crate::error::ErrorCode::InvalidSetting, "HTTP 验证端口无效"))?;
        if !(1..=65535).contains(&port) {
            return Err(AppError::new(
                crate::error::ErrorCode::InvalidSetting,
                "HTTP 验证端口必须在 1-65535 之间",
            ));
        }
    }
    if key == "acme_directory" && value != "staging" && value != "production" {
        return Err(AppError::new(
            crate::error::ErrorCode::InvalidSetting,
            "申请环境仅支持 staging / production",
        ));
    }
    if key == "cert_key_type" && value != "rsa" && value != "ecc" {
        return Err(AppError::new(
            crate::error::ErrorCode::InvalidSetting,
            "证书密钥类型仅支持 rsa / ecc",
        ));
    }

    let conn = state.db.lock();
    crate::storage::settings::set(&conn, &key, &value)?;
    drop(conn);

    log::info!("setting changed: {key} = {value}");

    // 开机自启开关即时生效（无需重启应用）
    if key == "run_at_login" {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        if value == "true" {
            let _ = autolaunch.enable();
        } else {
            let _ = autolaunch.disable();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_settings(
    settings: Settings,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    // 设置值校验
    if !(1..=65535).contains(&settings.http01_port) {
        return Err(AppError::new(
            crate::error::ErrorCode::InvalidSetting,
            "HTTP 验证端口必须在 1-65535 之间",
        ));
    }
    if settings.acme_directory != "staging" && settings.acme_directory != "production" {
        return Err(AppError::new(
            crate::error::ErrorCode::InvalidSetting,
            "申请环境仅支持 staging / production",
        ));
    }
    if settings.cert_key_type != "rsa" && settings.cert_key_type != "ecc" {
        return Err(AppError::new(
            crate::error::ErrorCode::InvalidSetting,
            "证书密钥类型仅支持 rsa / ecc",
        ));
    }

    let conn = state.db.lock();
    crate::storage::settings::set(&conn, "acme_directory", &settings.acme_directory)?;
    crate::storage::settings::set(&conn, "contact_email", &settings.contact_email)?;
    crate::storage::settings::set(&conn, "auto_renew", &settings.auto_renew.to_string())?;
    crate::storage::settings::set(&conn, "run_at_login", &settings.run_at_login.to_string())?;
    crate::storage::settings::set(&conn, "http01_port", &settings.http01_port.to_string())?;
    crate::storage::settings::set(
        &conn,
        "default_provider_id",
        &settings.default_provider_id.map(|v| v.to_string()).unwrap_or_default(),
    )?;
    crate::storage::settings::set(&conn, "cert_key_type", &settings.cert_key_type)?;
    crate::storage::settings::set(&conn, "notify_expiring", &settings.notify_expiring.to_string())?;
    crate::storage::settings::set(&conn, "notify_renew_success", &settings.notify_renew_success.to_string())?;
    crate::storage::settings::set(&conn, "notify_renew_failed", &settings.notify_renew_failed.to_string())?;
    drop(conn);

    log::info!(
        "settings saved: directory={} http01_port={} auto_renew={} run_at_login={}",
        settings.acme_directory,
        settings.http01_port,
        settings.auto_renew,
        settings.run_at_login
    );

    // 开机自启即时生效
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if settings.run_at_login {
        let _ = autolaunch.enable();
    } else {
        let _ = autolaunch.disable();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归：Settings 必须蛇形序列化（与前端 stores/settings.ts 一致）
    #[test]
    fn settings_serialize_snake_case() {
        let s = Settings {
            acme_directory: "staging".into(),
            contact_email: "a@b.com".into(),
            auto_renew: true,
            run_at_login: false,
            http01_port: 80,
            default_provider_id: Some(1),
            cert_key_type: "rsa".into(),
            notify_expiring: true,
            notify_renew_success: false,
            notify_renew_failed: true,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("http01_port").is_some());
        assert!(v.get("http01Port").is_none());
        assert!(v.get("default_provider_id").is_some());
        // 反向：蛇形 JSON 可反序列化
        let s2: Settings = serde_json::from_value(v).unwrap();
        assert_eq!(s2.contact_email, "a@b.com");
    }
}
