// 诊断工具：只读导出应用数据库内容（排查用，可随时删除）
// 用法: cargo run --example dump_db -- <db路径>
fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_db <path>");
    let conn = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();

    println!("--- tables ---");
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap();
    for t in stmt.query_map([], |r| r.get::<_, String>(0)).unwrap() {
        println!("  {}", t.unwrap());
    }

    println!("--- settings ---");
    let mut stmt = conn
        .prepare("SELECT key, value, updated_at FROM settings ORDER BY key")
        .unwrap();
    for row in stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })
        .unwrap()
    {
        println!("  {:?}", row.unwrap());
    }

    println!("--- certificates ---");
    let mut stmt = conn
        .prepare("SELECT id, domain, directory, status, expires_at FROM certificates ORDER BY id")
        .unwrap();
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .unwrap()
    {
        println!("  {:?}", row.unwrap());
    }

    println!("--- dns_providers ---");
    let mut stmt = conn
        .prepare("SELECT id, kind, label, enabled FROM dns_providers ORDER BY id")
        .unwrap();
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
    {
        println!("  {:?}", row.unwrap());
    }

    println!("--- job_logs (last 5) ---");
    let mut stmt = conn
        .prepare("SELECT id, job_type, target, status, error_code, started_at, finished_at FROM job_logs ORDER BY id DESC LIMIT 5")
        .unwrap();
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })
        .unwrap()
    {
        println!("  {:?}", row.unwrap());
    }
}
