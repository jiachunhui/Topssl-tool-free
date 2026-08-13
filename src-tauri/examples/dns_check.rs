// 诊断工具：用新的 wait_propagation（Cloudflare 公共 DNS）实测真实记录（排查用，可随时删除）
// 用法: cargo run --example dns_check -- <记录名> <TXT值>
fn main() {
    let record = std::env::args().nth(1).expect("usage: dns_check <record_name> <value>");
    let value = std::env::args().nth(2).expect("missing value");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let start = std::time::Instant::now();
    let r = rt.block_on(ssl_cert_desktop_lib::dns::wait_propagation(
        &record,
        &value,
        std::time::Duration::from_secs(30),
    ));
    println!(
        "[dns] record={record} found={} elapsed={:?}",
        r.is_ok(),
        start.elapsed()
    );
    if let Err(e) = r {
        println!("[dns] error: {e}");
    }
}
