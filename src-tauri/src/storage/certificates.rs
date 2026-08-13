//! 证书记录表操作

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CertStatus {
    Issued,
    Renewing,
    Failed,
    Expired,
    Revoked,
}

impl CertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CertStatus::Issued => "issued",
            CertStatus::Renewing => "renewing",
            CertStatus::Failed => "failed",
            CertStatus::Expired => "expired",
            CertStatus::Revoked => "revoked",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "renewing" => CertStatus::Renewing,
            "failed" => CertStatus::Failed,
            "expired" => CertStatus::Expired,
            "revoked" => CertStatus::Revoked,
            _ => CertStatus::Issued,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CertRow {
    pub id: i64,
    pub domain: String,
    pub alt_names: Vec<String>,
    pub challenge_type: String,
    pub provider_id: Option<i64>,
    pub directory: String,
    pub status: CertStatus,
    pub cert_chain_path: String,
    pub private_key_path: String,
    pub issuer: Option<String>,
    pub issued_at: String,
    pub expires_at: String,
    pub renew_after: Option<String>,
    pub last_renewal_at: Option<String>,
    pub last_error: Option<String>,
    pub order_url: Option<String>,
}

fn row_to_cert(r: &Row) -> rusqlite::Result<CertRow> {
    let alt_json: String = r.get("alt_names")?;
    let alt_names: Vec<String> = serde_json::from_str(&alt_json).unwrap_or_default();
    Ok(CertRow {
        id: r.get("id")?,
        domain: r.get("domain")?,
        alt_names,
        challenge_type: r.get("challenge_type")?,
        provider_id: r.get("provider_id")?,
        directory: r.get("directory")?,
        status: CertStatus::from_str(&r.get::<_, String>("status")?),
        cert_chain_path: r.get("cert_chain_path")?,
        private_key_path: r.get("private_key_path")?,
        issuer: r.get("issuer")?,
        issued_at: r.get("issued_at")?,
        expires_at: r.get("expires_at")?,
        renew_after: r.get("renew_after")?,
        last_renewal_at: r.get("last_renewal_at")?,
        last_error: r.get("last_error")?,
        order_url: r.get("order_url")?,
    })
}

const COLS: &str = "id, domain, alt_names, challenge_type, provider_id, directory, status, cert_chain_path, private_key_path, issuer, issued_at, expires_at, renew_after, last_renewal_at, last_error, order_url";

/// 插入列（不含自增 id，与 INSERT 的 15 个占位符一一对应）
const INSERT_COLS: &str = "domain, alt_names, challenge_type, provider_id, directory, status, cert_chain_path, private_key_path, issuer, issued_at, expires_at, renew_after, last_renewal_at, last_error, order_url";

pub fn insert(conn: &Connection, c: &CertRow) -> rusqlite::Result<i64> {
    conn.execute(
        &format!("INSERT INTO certificates ({INSERT_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)"),
        rusqlite::params![
            c.domain,
            serde_json::to_string(&c.alt_names).unwrap_or("[]".into()),
            c.challenge_type,
            c.provider_id,
            c.directory,
            c.status.as_str(),
            c.cert_chain_path,
            c.private_key_path,
            c.issuer,
            c.issued_at,
            c.expires_at,
            c.renew_after,
            c.last_renewal_at,
            c.last_error,
            c.order_url,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<CertRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM certificates ORDER BY created_at DESC"))?;
    let rows = stmt.query_map([], row_to_cert)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<CertRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM certificates WHERE id=?1"))?;
    let mut rows = stmt.query_map([id], row_to_cert)?;
    rows.next().transpose()
}

pub fn get_by_domain(conn: &Connection, domain: &str, directory: &str) -> rusqlite::Result<Option<CertRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM certificates WHERE domain=?1 AND directory=?2 ORDER BY id DESC"))?;
    let mut rows = stmt.query_map([domain, directory], row_to_cert)?;
    rows.next().transpose()
}

/// 引用某 DNS 服务商的证书数量（删除服务商前检查）
pub fn count_by_provider(conn: &Connection, provider_id: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM certificates WHERE provider_id=?1",
        [provider_id],
        |r| r.get(0),
    )
}

/// 启动恢复：应用退出时内存中的续期任务会中断，把卡在 renewing 的证书回滚为 issued，
/// 下次调度检查会自动重试
pub fn reset_interrupted_renewals(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE certificates SET status='issued',
         last_error='续期任务中断（应用曾退出），将在下次检查时自动重试', updated_at=datetime('now')
         WHERE status='renewing'",
        [],
    )
}

pub fn update_status(conn: &Connection, id: i64, status: CertStatus, last_error: Option<&str>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE certificates SET status=?1, last_error=?2, updated_at=datetime('now') WHERE id=?3",
        rusqlite::params![status.as_str(), last_error, id],
    )?;
    Ok(())
}

pub fn update_after_renew(
    conn: &Connection,
    id: i64,
    cert_chain_path: &str,
    private_key_path: &str,
    expires_at: &str,
    renew_after: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE certificates SET cert_chain_path=?1, private_key_path=?2, expires_at=?3, renew_after=?4,
         last_renewal_at=datetime('now'), status='issued', last_error=NULL, updated_at=datetime('now') WHERE id=?5",
        rusqlite::params![cert_chain_path, private_key_path, expires_at, renew_after, id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM certificates WHERE id=?1", [id])?;
    Ok(())
}

/// 需要续期的证书：issued 且 expires_at < now + 30d
pub fn list_due_renewal(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<Vec<CertRow>> {
    let threshold = (now + chrono::Duration::days(30)).to_rfc3339();
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM certificates WHERE status='issued' AND expires_at < ?1 ORDER BY expires_at"
    ))?;
    let rows = stmt.query_map([threshold], row_to_cert)?;
    rows.collect()
}
