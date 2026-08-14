//! DNS Provider 配置表操作

use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Aliyun,
    Dnspod,
    Cloudflare,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Aliyun => "aliyun",
            ProviderKind::Dnspod => "dnspod",
            ProviderKind::Cloudflare => "cloudflare",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "aliyun" => ProviderKind::Aliyun,
            "cloudflare" => ProviderKind::Cloudflare,
            _ => ProviderKind::Dnspod,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderRow {
    pub id: i64,
    pub kind: ProviderKind,
    pub label: String,
    pub config_json: String, // 非机密配置
    pub secret_ref: String,
    pub enabled: bool,
    pub created_at: String,
}

fn row_to_provider(r: &Row) -> rusqlite::Result<ProviderRow> {
    Ok(ProviderRow {
        id: r.get("id")?,
        kind: ProviderKind::from_str(&r.get::<_, String>("kind")?),
        label: r.get("label")?,
        config_json: r.get("config_json")?,
        secret_ref: r.get("secret_ref")?,
        enabled: r.get::<_, i64>("enabled")? != 0,
        created_at: r.get("created_at")?,
    })
}

const COLS: &str = "id, kind, label, config_json, secret_ref, enabled, created_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<ProviderRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM dns_providers ORDER BY id"))?;
    let rows = stmt.query_map([], row_to_provider)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<ProviderRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM dns_providers WHERE id=?1"))?;
    let mut rows = stmt.query_map([id], row_to_provider)?;
    rows.next().transpose()
}

pub fn insert(
    conn: &Connection,
    kind: &ProviderKind,
    label: &str,
    config_json: &str,
    secret_ref: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO dns_providers (kind, label, config_json, secret_ref, enabled) VALUES (?1,?2,?3,?4,1)",
        rusqlite::params![kind.as_str(), label, config_json, secret_ref],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    id: i64,
    kind: &ProviderKind,
    label: &str,
    config_json: &str,
    secret_ref: Option<&str>,
) -> rusqlite::Result<()> {
    match secret_ref {
        Some(secret) => conn.execute(
            "UPDATE dns_providers SET kind=?1, label=?2, config_json=?3, secret_ref=?4, updated_at=datetime('now') WHERE id=?5",
            rusqlite::params![kind.as_str(), label, config_json, secret, id],
        )?,
        None => conn.execute(
            "UPDATE dns_providers SET kind=?1, label=?2, config_json=?3, updated_at=datetime('now') WHERE id=?4",
            rusqlite::params![kind.as_str(), label, config_json, id],
        )?,
    };
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM dns_providers WHERE id=?1", [id])?;
    Ok(())
}
