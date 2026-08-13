//! 数据库增量迁移（PRAGMA user_version 驱动）

use rusqlite::Connection;

pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(V1)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}

const V1: &str = r#"
CREATE TABLE IF NOT EXISTS dns_providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  label TEXT NOT NULL DEFAULT '',
  config_json TEXT NOT NULL DEFAULT '{}',
  secret_ref TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_provider_kind ON dns_providers(kind);

CREATE TABLE IF NOT EXISTS certificates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  domain TEXT NOT NULL,
  alt_names TEXT NOT NULL DEFAULT '[]',
  challenge_type TEXT NOT NULL,
  provider_id INTEGER REFERENCES dns_providers(id),
  directory TEXT NOT NULL DEFAULT 'staging',
  status TEXT NOT NULL DEFAULT 'issued',
  cert_chain_path TEXT NOT NULL,
  private_key_path TEXT NOT NULL,
  issuer TEXT,
  issued_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  renew_after TEXT,
  last_renewal_at TEXT,
  last_error TEXT,
  order_url TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_certs_domain ON certificates(domain);
CREATE INDEX IF NOT EXISTS idx_certs_status ON certificates(status);
CREATE INDEX IF NOT EXISTS idx_certs_expires ON certificates(expires_at);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS job_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_type TEXT NOT NULL,
  target TEXT,
  status TEXT NOT NULL,
  error_code TEXT,
  detail TEXT,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_logs ON job_logs(job_type, started_at);
"#;
