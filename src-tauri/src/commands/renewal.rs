//! 续期检查 command：check_renewals / set_auto_renew

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct RenewalResult {
    pub cert_id: i64,
    pub domain: String,
    pub ok: bool,
    pub message: String,
}

/// Tauri command 包装
#[tauri::command]
pub fn check_renewals(
    force: bool,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<Vec<RenewalResult>> {
    check_renewals_impl(force, &state, app)
}

/// 检查需要续期的证书并触发续期（到期前 30 天）
pub fn check_renewals_impl(
    force: bool,
    state: &AppState,
    app: tauri::AppHandle,
) -> AppResult<Vec<RenewalResult>> {
    let auto_renew = crate::storage::settings::get_bool(&state.db.lock(), "auto_renew", true);
    if !auto_renew && !force {
        return Ok(vec![]);
    }

    let conn = state.db.lock();
    let due = crate::storage::certificates::list_due_renewal(&conn, Utc::now())?;
    drop(conn);
    log::info!("renewal check: {} cert(s) due", due.len());

    let mut results = Vec::new();
    for cert in due {
        // 检查是否已有正在运行的续期任务（含刚创建还未启动的 Pending）
        let running = state
            .jobs
            .lock()
            .unwrap()
            .values()
            .any(|j| {
                j.state == crate::acme::model::JobState::Running
                    || j.state == crate::acme::model::JobState::Pending
            });
        if running {
            results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: false,
                message: "已有任务进行中".into(),
            });
            continue;
        }

        let req = crate::acme::model::IssueRequest {
            domain: cert.domain.clone(),
            alt_names: cert.alt_names.clone(),
            challenge_type: cert.challenge_type.clone(),
            provider_id: cert.provider_id,
            dns_manual: false,
            directory: cert.directory.clone(),
            contact_email: crate::storage::settings::get_string(&state.db.lock(), "contact_email", ""),
        };

        match crate::commands::issue::spawn_issue_job(state, app.clone(), req, Some(cert.id)) {
            Ok(_) => results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: true,
                message: "已触发续期".into(),
            }),
            Err(e) => results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: false,
                message: e.message,
            }),
        }
    }

    // 记录检查时间
    let now = Utc::now().to_rfc3339();
    let _ = crate::storage::settings::set(&state.db.lock(), "last_check_at", &now);

    Ok(results)
}

/// 到期天数（供调度器判断）
pub fn days_left(expires_at: &str) -> i64 {
    let Ok(dt) = DateTime::parse_from_rfc3339(expires_at) else { return 0 };
    (dt.with_timezone(&Utc) - Utc::now()).num_days()
}
