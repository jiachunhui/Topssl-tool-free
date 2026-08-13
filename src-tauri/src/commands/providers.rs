//! DNS Provider 配置 command

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::storage::providers::{ProviderKind, ProviderRow};

/// 字段用蛇形命名序列化（与前端 types.ts 的 ProviderInfo 接口一一对应）
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub kind: String,
    pub label: String,
    pub config: serde_json::Map<String, serde_json::Value>,
}

/// 机密字段（存入 keyring 的 JSON 键）
const SECRET_KEYS: [&str; 3] = ["access_key_secret", "login_token", "api_token"];

fn row_to_info(row: &ProviderRow) -> ProviderInfo {
    let config: serde_json::Value = serde_json::from_str(&row.config_json).unwrap_or(serde_json::json!({}));
    ProviderInfo {
        id: row.id,
        kind: row.kind.as_str().to_string(),
        label: row.label.clone(),
        config,
        enabled: row.enabled,
        created_at: row.created_at.clone(),
    }
}

#[tauri::command]
pub fn list_providers(state: tauri::State<'_, AppState>) -> AppResult<Vec<ProviderInfo>> {
    let conn = state.db.lock();
    let rows = crate::storage::providers::list(&conn)?;
    Ok(rows.into_iter().map(|r| row_to_info(&r)).collect())
}

/// 新增或更新 Provider（机密字段加密入 keyring）
#[tauri::command]
pub fn save_provider(cfg: ProviderInput, state: tauri::State<'_, AppState>) -> AppResult<i64> {
    let kind = match cfg.kind.as_str() {
        "aliyun" => ProviderKind::Aliyun,
        "cloudflare" => ProviderKind::Cloudflare,
        "dnspod" => ProviderKind::Dnspod,
        other => {
            return Err(AppError::new(
                crate::error::ErrorCode::InvalidSetting,
                format!("未知的 DNS 服务商类型: {other}"),
            ));
        }
    };

    // 分离机密字段
    let mut non_secret = serde_json::Map::new();
    let mut secret = serde_json::Map::new();
    for (k, v) in &cfg.config {
        if SECRET_KEYS.contains(&k.as_str()) {
            secret.insert(k.clone(), v.clone());
        } else {
            non_secret.insert(k.clone(), v.clone());
        }
    }
    let config_json = serde_json::to_string(&non_secret)?;

    let conn = state.db.lock();
    let id = match cfg.id {
        Some(id) => {
            // 更新：若提供了新机密则覆盖
            let existing = crate::storage::providers::get(&conn, id)?.ok_or_else(|| {
                AppError::new(crate::error::ErrorCode::Db, "Provider 不存在")
            })?;
            if !secret.is_empty() {
                state.secrets.save(&existing.secret_ref, &serde_json::to_string(&secret)?)?;
                crate::storage::providers::update(&conn, id, &kind, &cfg.label, &config_json, Some(&existing.secret_ref))?;
            } else {
                crate::storage::providers::update(&conn, id, &kind, &cfg.label, &config_json, None)?;
            }
            id
        }
        None => {
            let secret_ref = format!("dns_provider:{}", uuid::Uuid::new_v4());
            state.secrets.save(&secret_ref, &serde_json::to_string(&secret)?)?;
            crate::storage::providers::insert(&conn, &kind, &cfg.label, &config_json, &secret_ref)?
        }
    };
    drop(conn);
    log::info!("provider saved: id={id} kind={} label={}", kind.as_str(), cfg.label);
    Ok(id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    pub ok: bool,
    pub message: String,
    pub zone: Option<String>,
}

/// 测试 Provider：验证凭证有效且能访问域名区域
#[tauri::command]
pub fn test_provider(id: i64, state: tauri::State<'_, AppState>) -> AppResult<ProviderTestResult> {
    let conn = state.db.lock();
    let row = crate::storage::providers::get(&conn, id)?
        .ok_or_else(|| AppError::new(crate::error::ErrorCode::Db, "Provider 不存在"))?;
    drop(conn);

    let provider = crate::dns::build_provider(&row, &state.secrets)?;
    let result = tauri::async_runtime::block_on(async { provider.test("").await });
    log::info!("provider test: id={id} ok={}", result.is_ok());
    if let Err(e) = &result {
        log::warn!("provider test failed: id={id} code={} message={} detail={:?}", e.code.as_str(), e.message, e.detail);
    }
    Ok(match result {
        Ok(()) => ProviderTestResult { ok: true, message: "配置有效，可正常管理解析记录".into(), zone: None },
        Err(e) => ProviderTestResult {
            ok: false,
            message: match e.code {
                crate::error::ErrorCode::DnsProviderAuth => e.message,
                crate::error::ErrorCode::DnsTxtNotFound => "凭证有效，但账户下未找到域名区域".into(),
                _ => e.message,
            },
            zone: None,
        },
    })
}

#[tauri::command]
pub fn delete_provider(id: i64, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock();
    let row = crate::storage::providers::get(&conn, id)?;
    let Some(r) = row else { return Ok(()) };

    // 有证书引用时拒绝删除（外键约束会失败，且删除后这些证书的续期会静默失效）
    let in_use = crate::storage::certificates::count_by_provider(&conn, id)?;
    if in_use > 0 {
        return Err(AppError::new(
            crate::error::ErrorCode::DnsProviderApi,
            format!("该服务商正被 {in_use} 张证书使用，请先删除这些证书"),
        ));
    }

    // 先删数据库记录，成功后再清理 keyring（避免先删密钥导致 DB 删除失败时机密丢失）
    crate::storage::providers::delete(&conn, id)?;
    drop(conn);
    state.secrets.delete(&r.secret_ref)?;
    log::info!("provider deleted: id={id} label={}", r.label);
    Ok(())
}
