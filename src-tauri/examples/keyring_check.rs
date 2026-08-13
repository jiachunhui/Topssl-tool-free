// 诊断工具：读回 keyring 中已保存的阿里云密钥（脱敏）+ keyring 往返完整性测试（排查用）
// 用法: cargo run --example keyring_check -- <db路径> [provider_id]
fn main() {
    let mut args = std::env::args().skip(1);
    let db_path = args.next().expect("usage: keyring_check <db_path> [provider_id]");
    let provider_id: i64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();

    let secret_ref: String = conn
        .query_row("SELECT secret_ref FROM dns_providers WHERE id=?1", [provider_id], |r| r.get(0))
        .unwrap();
    println!("[keyring] provider id={provider_id} secret_ref: {secret_ref}");

    let entry = keyring::Entry::new("ssl-cert-desktop", &secret_ref).unwrap();
    match entry.get_password() {
        Ok(p) => {
            println!("[keyring] stored json len: {}", p.len());
            let v: serde_json::Value = serde_json::from_str(&p).unwrap_or_default();
            if let Some(s) = v.get("access_key_secret").and_then(|x| x.as_str()) {
                let c: Vec<char> = s.chars().collect();
                let head: String = c.iter().take(2).collect();
                let tail: String = c.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
                println!("[keyring] secret len: {}  head: {head}  tail: {tail}  ascii: {}", s.len(), s.is_ascii());
            } else {
                println!("[keyring] 未找到 access_key_secret 字段，原始内容: {p}");
            }
        }
        Err(e) => println!("[keyring] read error: {e}"),
    }

    // 往返完整性：写已知值再读回（含特殊字符与中文）
    let test_key = "dns_provider:__roundtrip_test";
    let test_val = "abc123XYZ!@#%^&*() 测试";
    let e2 = keyring::Entry::new("ssl-cert-desktop", test_key).unwrap();
    e2.set_password(test_val).unwrap();
    let back = e2.get_password().unwrap();
    println!("[keyring] roundtrip ok: {}", back == test_val);
    let _ = e2.delete_credential();
}
