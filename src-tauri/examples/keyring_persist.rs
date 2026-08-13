// 诊断工具：SecretStore 跨进程持久化测试（排查用，可随时删除）
// 用法: cargo run --example keyring_persist -- write|read|delete [存储目录]
fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "write".into());
    let dir = args
        .next()
        .unwrap_or_else(|| std::env::temp_dir().to_string_lossy().into_owned());
    let store = ssl_cert_desktop_lib::secret::keyring::SecretStore::new(std::path::Path::new(&dir));
    let key = "dns_provider:__persist_test";
    match mode.as_str() {
        "write" => {
            let val = format!(
                "persist-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );
            match store.save(key, &val) {
                Ok(()) => println!("[persist] write ok: {val}"),
                Err(e) => println!("[persist] write error: {e}"),
            }
            match store.load(key) {
                Ok(Some(p)) => println!("[persist] immediate read ok (match={})", p == val),
                Ok(None) => println!("[persist] immediate read: None"),
                Err(e) => println!("[persist] immediate read error: {e}"),
            }
        }
        "read" => match store.load(key) {
            Ok(Some(p)) => println!("[persist] delayed read ok: len={} head={}", p.len(), &p[..8.min(p.len())]),
            Ok(None) => println!("[persist] delayed read: None"),
            Err(e) => println!("[persist] delayed read error: {e}"),
        },
        "delete" => {
            let _ = store.delete(key);
            println!("[persist] deleted");
        }
        _ => println!("[persist] unknown mode"),
    }
}
