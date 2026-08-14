//! 任务日志表操作

use chrono::Utc;
use rusqlite::Connection;

pub fn start(conn: &Connection, job_type: &str, target: Option<&str>) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO job_logs (job_type, target, status, started_at) VALUES (?1,?2,'running',?3)",
        rusqlite::params![job_type, target, Utc::now().to_rfc3339()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn finish(conn: &Connection, id: i64, status: &str, error_code: Option<&str>, detail: Option<&str>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE job_logs SET status=?1, error_code=?2, detail=?3, finished_at=?4 WHERE id=?5",
        rusqlite::params![status, error_code, detail, Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

/// 最近一次同目标【失败】任务的结束时间（用于冷却检查）
/// 时间戳为 RFC3339（UTC），与 limits.rs 的 parse_from_rfc3339 匹配
/// 注意：不消耗 LE 验证配额的失败不计入冷却：
///   - DNS 传播超时（含手动模式等待超时）
///   - 订单请求错误（如域名冗余被拒，修正后可立即重试）
///   - HTTP-01 监听失败（端口被占用 / 无权限，发生在发起验证前，
///     修正端口占用或改用 DNS 验证后应可立即重试）
pub fn last_finished_at(conn: &Connection, job_type: &str, target: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT finished_at FROM job_logs WHERE job_type=?1 AND target=?2 AND status='failed' \
         AND (error_code IS NULL OR error_code NOT IN \
              ('ERR_DNS_PROPAGATION_TIMEOUT','ERR_ORDER_CREATE','ERR_HTTP01_PRIVILEGE','ERR_HTTP01_PORT_BUSY')) \
         AND finished_at IS NOT NULL ORDER BY id DESC LIMIT 1",
        rusqlite::params![job_type, target],
        |r| r.get(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}
