//! 存储层：SQLite 连接管理 + 增量迁移

pub mod certificates;
pub mod logs;
pub mod migrations;
pub mod providers;
pub mod settings;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

pub struct Db {
    pub conn: Mutex<Connection>,
}

impl Db {
    /// 打开（或创建）数据库并执行迁移
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrations::run(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_and_failed_log_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();

        let id = logs::start(&conn, "issue", Some("example.com")).unwrap();
        logs::finish(&conn, id, "failed", Some("ERR_VALIDATION_FAILED"), Some("dns problem")).unwrap();

        let ts = logs::last_finished_at(&conn, "issue", "example.com").unwrap().unwrap();
        // 关键回归点：finished_at 必须是 RFC3339（此前 datetime('now') 格式导致冷却机制失效）
        chrono::DateTime::parse_from_rfc3339(&ts).unwrap();

        // 冷却只统计失败任务：成功任务不应覆盖失败时间戳
        let id2 = logs::start(&conn, "issue", Some("example.com")).unwrap();
        logs::finish(&conn, id2, "completed", None, None).unwrap();
        assert_eq!(logs::last_finished_at(&conn, "issue", "example.com").unwrap().unwrap(), ts);
    }
}
