//! 申请护栏：重复申请检测 / 失败冷却

use chrono::{DateTime, Utc};

use crate::acme::model::IssueRequest;
use crate::error::{AppError, ErrorCode};
use crate::storage::certificates::CertStatus;
use crate::storage::Db;

/// 检查同环境下是否已有有效证书（剩余 > 30 天则拦截）
/// directory 用于区分 staging / production 证书互不干扰
pub fn check_duplicate(db: &Db, domain: &str, directory: &str) -> Result<Option<i64>, AppError> {
    let conn = db.lock();
    let existing = crate::storage::certificates::get_by_domain(&conn, domain, directory)?;
    if let Some(c) = existing {
        if c.status == CertStatus::Issued {
            if let Ok(expires) = DateTime::parse_from_rfc3339(&c.expires_at) {
                let days = (expires.with_timezone(&Utc) - Utc::now()).num_days();
                if days > 30 {
                    return Ok(Some(c.id));
                }
            }
        }
    }
    Ok(None)
}

/// 冷却检查：同域上次【失败】任务距今不足 10 分钟则拦截
pub fn check_cooldown(db: &Db, domain: &str) -> Result<(), AppError> {
    let conn = db.lock();
    if let Some(finished) = crate::storage::logs::last_finished_at(&conn, "issue", domain)? {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&finished) {
            if (Utc::now() - dt.with_timezone(&Utc)).num_minutes() < 10 {
                return Err(AppError::new(
                    ErrorCode::CoolDown,
                    "申请过于频繁，请 10 分钟后再试",
                ));
            }
        }
    }
    Ok(())
}

/// 申请前完整护栏校验
/// is_renewal：续期任务跳过重复证书拦截（续期对象本身就是有效证书）
pub fn preflight(db: &Db, req: &IssueRequest, is_renewal: bool) -> Result<(), AppError> {
    check_cooldown(db, &req.domain)?;
    if !is_renewal {
        if let Some(cert_id) = check_duplicate(db, &req.domain, &req.directory)? {
            return Err(AppError::new(
                ErrorCode::DuplicateCert,
                "该域名已有有效证书（距到期超过 30 天），可直接对现有证书执行续期",
            )
            .detail(format!("cert_id={cert_id}")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::certificates::CertRow;

    fn test_db() -> (Db, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("ssl_test_{}.db", uuid::Uuid::new_v4()));
        let db = Db::open(&path).unwrap();
        (db, path)
    }

    fn insert_cert(db: &Db, domain: &str, directory: &str, days: i64) {
        let conn = db.lock();
        let row = CertRow {
            id: 0,
            domain: domain.into(),
            alt_names: vec![],
            challenge_type: "dns01".into(),
            provider_id: None,
            directory: directory.into(),
            status: CertStatus::Issued,
            cert_chain_path: "/tmp/fullchain.pem".into(),
            private_key_path: "/tmp/privkey.pem".into(),
            issuer: None,
            issued_at: Utc::now().to_rfc3339(),
            expires_at: (Utc::now() + chrono::Duration::days(days)).to_rfc3339(),
            renew_after: None,
            last_renewal_at: None,
            last_error: None,
            order_url: None,
        };
        crate::storage::certificates::insert(&conn, &row).unwrap();
    }

    #[test]
    fn cooldown_blocks_recent_failure() {
        let (db, path) = test_db();
        let conn = db.lock();
        let id = crate::storage::logs::start(&conn, "issue", Some("example.com")).unwrap();
        crate::storage::logs::finish(&conn, id, "failed", None, None).unwrap();
        drop(conn);

        assert!(check_cooldown(&db, "example.com").is_err());
        // 其他域名不受影响
        assert!(check_cooldown(&db, "other.com").is_ok());

        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cooldown_ignores_success() {
        let (db, path) = test_db();
        let conn = db.lock();
        let id = crate::storage::logs::start(&conn, "issue", Some("example.com")).unwrap();
        crate::storage::logs::finish(&conn, id, "completed", None, None).unwrap();
        drop(conn);

        assert!(check_cooldown(&db, "example.com").is_ok());

        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn duplicate_only_within_same_directory() {
        let (db, path) = test_db();
        insert_cert(&db, "example.com", "production", 60);

        // 同环境：拦截
        assert!(check_duplicate(&db, "example.com", "production").unwrap().is_some());
        // 不同环境：互不干扰
        assert!(check_duplicate(&db, "example.com", "staging").unwrap().is_none());
        // 剩余不足 30 天：允许重新申请
        insert_cert(&db, "old.com", "production", 10);
        assert!(check_duplicate(&db, "old.com", "production").unwrap().is_none());

        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
