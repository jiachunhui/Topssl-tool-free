//! 通知去重记录表操作（防骚扰：同一证书同一提醒级别只发一次）

use rusqlite::Connection;

/// 该证书该级别是否已通知过
pub fn exists(conn: &Connection, kind: &str, cert_id: i64, level: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM notifications WHERE kind=?1 AND cert_id=?2 AND level=?3",
        rusqlite::params![kind, cert_id, level],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

/// 记录一条已发送的通知（作为去重标记）
pub fn record(conn: &Connection, kind: &str, cert_id: i64, level: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO notifications (kind, cert_id, level) VALUES (?1, ?2, ?3)",
        rusqlite::params![kind, cert_id, level],
    )?;
    Ok(())
}
