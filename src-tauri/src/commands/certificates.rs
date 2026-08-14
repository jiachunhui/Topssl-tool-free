//! 证书类 command

use serde::Serialize;

use crate::cert::parser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::storage::certificates::{CertRow, CertStatus};

/// 字段用蛇形命名序列化（与前端 types.ts 的 CertInfo 接口一一对应）
#[derive(Debug, Serialize)]
pub struct CertInfo {
    pub id: i64,
    pub domain: String,
    pub alt_names: Vec<String>,
    pub challenge_type: String,
    pub provider_id: Option<i64>,
    pub directory: String,
    pub status: String,
    pub cert_chain_path: String,
    pub private_key_path: String,
    pub issuer: Option<String>,
    pub issued_at: String,
    pub expires_at: String,
    pub days_remaining: i64,
    pub renew_after: Option<String>,
    pub last_renewal_at: Option<String>,
    pub last_error: Option<String>,
    pub order_url: Option<String>,
}

impl From<CertRow> for CertInfo {
    fn from(c: CertRow) -> Self {
        let days = parser::days_remaining(&c.expires_at);
        // 到期后以 expired 展示（数据库仍为 issued，调度器按 expires_at 判断续期）。
        // 仅当日期可解析且确实已过期时才标 expired；
        // 解析失败（days_remaining 返回 0）不再误标，避免有效证书显示为已过期（轻微问题 2）
        let expired = c.status == CertStatus::Issued
            && days <= 0
            && chrono::DateTime::parse_from_rfc3339(&c.expires_at).is_ok();
        let status = if expired {
            CertStatus::Expired
        } else {
            c.status
        };
        Self {
            id: c.id,
            domain: c.domain,
            alt_names: c.alt_names,
            challenge_type: c.challenge_type,
            provider_id: c.provider_id,
            directory: c.directory,
            status: status.as_str().to_string(),
            cert_chain_path: c.cert_chain_path,
            private_key_path: c.private_key_path,
            issuer: c.issuer,
            issued_at: c.issued_at,
            expires_at: c.expires_at,
            days_remaining: days.max(0),
            renew_after: c.renew_after,
            last_renewal_at: c.last_renewal_at,
            last_error: c.last_error,
            order_url: c.order_url,
        }
    }
}

#[tauri::command]
pub fn list_certificates(state: tauri::State<'_, AppState>) -> AppResult<Vec<CertInfo>> {
    let conn = state.db.lock();
    let rows = crate::storage::certificates::list(&conn)?;
    Ok(rows.into_iter().map(CertInfo::from).collect())
}

#[tauri::command]
pub fn get_certificate(id: i64, state: tauri::State<'_, AppState>) -> AppResult<Option<CertInfo>> {
    let conn = state.db.lock();
    Ok(crate::storage::certificates::get(&conn, id)?.map(CertInfo::from))
}

/// 删除证书记录与文件
#[tauri::command]
pub fn delete_certificate(id: i64, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock();
    let cert = crate::storage::certificates::get(&conn, id)?;
    if let Some(c) = cert {
        // 先删记录再删文件：文件删除失败只留孤儿文件（日志告警），
        // 避免出现记录指向已删除文件的悬空状态
        crate::storage::certificates::delete(&conn, id)?;
        drop(conn);
        if let Err(e) = crate::cert::store::remove_bundle(&state.certs_root, &c.domain) {
            log::error!("failed to remove cert files for {}: {e}", c.domain);
        } else {
            log::info!("certificate deleted: id={id} domain={}", c.domain);
        }
    }
    Ok(())
}

/// 生成使用指引
#[tauri::command]
pub fn get_usage_guide(id: i64, state: tauri::State<'_, AppState>) -> AppResult<String> {
    let conn = state.db.lock();
    let cert = crate::storage::certificates::get(&conn, id)?
        .ok_or_else(|| AppError::new(crate::error::ErrorCode::Db, "证书不存在"))?;
    Ok(crate::cert::guide::generate_guide(
        &cert.domain,
        &cert.cert_chain_path,
        &cert.private_key_path,
        &state.platform,
    ))
}

/// 检查域名是否已有有效证书（前端向导用，按环境区分）
#[tauri::command]
pub fn check_duplicate(
    domain: String,
    directory: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    let directory = directory.unwrap_or_else(|| "staging".into());
    let cert_id = crate::acme::limits::check_duplicate(&state.db, &domain, &directory)?;
    Ok(serde_json::json!({ "duplicate": cert_id.is_some(), "certId": cert_id }))
}

/// 立即续期（返回 job_id）
#[tauri::command]
pub fn renew_now(id: i64, state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> AppResult<String> {
    let conn = state.db.lock();
    let cert = crate::storage::certificates::get(&conn, id)?
        .ok_or_else(|| AppError::new(crate::error::ErrorCode::Db, "证书不存在"))?;

    // 并发防护：已有续期任务进行中
    if cert.status == CertStatus::Renewing {
        return Err(AppError::new(crate::error::ErrorCode::DuplicateCert, "该证书正在续期中，请稍候"));
    }

    // 构建续期请求（沿用原挑战方式与 provider）。
    // 手动 DNS 证书（无 provider）：续期走手动模式，等待用户在向导页添加并确认 TXT 记录（B1）。
    // 续期邮箱优先复用证书申请时的邮箱（B5），为空时回退到设置默认邮箱。
    let dns_manual = cert.challenge_type == "dns01" && cert.provider_id.is_none();
    let contact_email = cert
        .contact_email
        .clone()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| crate::storage::settings::get_string(&conn, "contact_email", ""));
    let req = crate::acme::model::IssueRequest {
        domain: cert.domain.clone(),
        alt_names: cert.alt_names.clone(),
        challenge_type: cert.challenge_type.clone(),
        provider_id: if dns_manual { None } else { cert.provider_id },
        dns_manual,
        directory: cert.directory.clone(),
        contact_email,
    };
    drop(conn);

    // 标记续期中
    let conn = state.db.lock();
    crate::storage::certificates::update_status(&conn, id, CertStatus::Renewing, None)?;
    drop(conn);

    // spawn 失败（如域名校验不过、DNS 服务商缺失）时回滚状态，
    // 避免证书永久卡在"续期中"（按钮隐藏 + 调度器跳过）
    match crate::commands::issue::spawn_issue_job(state.inner(), app, req, Some(id)) {
        Ok(job_id) => {
            log::info!("renew_now: cert={id} domain={} job={job_id}", cert.domain);
            Ok(job_id)
        }
        Err(e) => {
            let conn = state.db.lock();
            let _ = crate::storage::certificates::update_status(&conn, id, CertStatus::Issued, Some(&e.message));
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::certificates::{CertRow, CertStatus};

    /// 回归：CertInfo 必须蛇形序列化（与前端 types.ts 一致）
    #[test]
    fn cert_info_serializes_snake_case() {
        let row = CertRow {
            id: 1,
            domain: "example.com".into(),
            alt_names: vec![],
            challenge_type: "dns01".into(),
            provider_id: None,
            directory: "staging".into(),
            status: CertStatus::Issued,
            cert_chain_path: "/tmp/fullchain.pem".into(),
            private_key_path: "/tmp/privkey.pem".into(),
            issuer: None,
            issued_at: "2026-08-13T00:00:00+00:00".into(),
            expires_at: (chrono::Utc::now() + chrono::Duration::days(90)).to_rfc3339(),
            renew_after: None,
            last_renewal_at: None,
            last_error: None,
            fail_streak: 0,
            order_url: None,
            contact_email: None,
        };
        let info = CertInfo::from(row);
        let v = serde_json::to_value(&info).unwrap();
        assert!(v.get("cert_chain_path").is_some());
        assert!(v.get("certChainPath").is_none());
        assert!(v.get("days_remaining").is_some());
        assert!(v.get("challenge_type").is_some());
    }
}
