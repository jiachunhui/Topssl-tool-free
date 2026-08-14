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
        // 手动 DNS 模式证书无法无人值守自动续期（需要人工添加 TXT 记录），
        // 自动检查跳过并给出明确提示（B1）；用户可到证书列表点「立即续期」走手动流程。
        if cert.challenge_type == "dns01" && cert.provider_id.is_none() {
            results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: true,
                message: "该证书为手动 DNS 验证，请在证书列表点击「立即续期」并手动添加 TXT 记录".into(),
            });
            continue;
        }

        // 该证书是否已有进行中任务（spawn_issue_job 内还会在锁内复核，双保险，B2/B3）
        let running_for_cert = state
            .jobs
            .lock()
            .unwrap()
            .values()
            .any(|j| (j.state == crate::acme::model::JobState::Running || j.state == crate::acme::model::JobState::Pending) && j.cert_id == Some(cert.id));
        if running_for_cert {
            results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: false,
                message: "已有续期任务进行中".into(),
            });
            continue;
        }

        // 续期优先复用证书申请时的邮箱（B5），为空时才回退到设置默认邮箱
        let contact_email = cert
            .contact_email
            .clone()
            .filter(|e| !e.trim().is_empty())
            .unwrap_or_else(|| crate::storage::settings::get_string(&state.db.lock(), "contact_email", ""));

        let req = crate::acme::model::IssueRequest {
            domain: cert.domain.clone(),
            alt_names: cert.alt_names.clone(),
            challenge_type: cert.challenge_type.clone(),
            provider_id: cert.provider_id,
            dns_manual: false,
            directory: cert.directory.clone(),
            contact_email,
        };

        // 标记续期中（与手动「立即续期」一致；spawn 失败回滚，避免卡在 renewing）
        let conn = state.db.lock();
        if let Err(e) = crate::storage::certificates::update_status(
            &conn,
            cert.id,
            crate::storage::certificates::CertStatus::Renewing,
            None,
        ) {
            drop(conn);
            results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: false,
                message: e.to_string(),
            });
            continue;
        }
        drop(conn);

        match crate::commands::issue::spawn_issue_job(state, app.clone(), req, Some(cert.id)) {
            Ok(_) => results.push(RenewalResult {
                cert_id: cert.id,
                domain: cert.domain.clone(),
                ok: true,
                message: "已触发续期".into(),
            }),
            Err(e) => {
                let conn = state.db.lock();
                let _ = crate::storage::certificates::update_status(
                    &conn,
                    cert.id,
                    crate::storage::certificates::CertStatus::Issued,
                    Some(&e.message),
                );
                drop(conn);
                results.push(RenewalResult {
                    cert_id: cert.id,
                    domain: cert.domain.clone(),
                    ok: false,
                    message: e.message,
                });
            }
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
