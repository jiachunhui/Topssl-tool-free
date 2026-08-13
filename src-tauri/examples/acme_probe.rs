// 诊断工具：真实调用 Let's Encrypt Staging 复现 ACME 解析错误（排查用，可随时删除）
// 用法: cargo run --example acme_probe -- [域名]
// 开启 DEBUG 日志后，acme-micro 会把原始响应体打印到 app.log，可看到缺少 token 的挑战类型
fn main() {
    ssl_cert_desktop_lib::logs::init();
    log::set_max_level(log::LevelFilter::Debug);
    let dir = std::env::temp_dir();
    ssl_cert_desktop_lib::logs::set_file(&dir);

    let email = "probe@wzjs10.com";
    let d = ssl_cert_desktop_lib::acme::client::connect("staging").unwrap();
    println!("[probe] directory ok");

    let pem = ssl_cert_desktop_lib::acme::client::register_account(&d, email).unwrap();
    println!("[probe] account registered");
    let acc = ssl_cert_desktop_lib::acme::client::load_account(&d, &pem, email).unwrap();

    let domain = std::env::args().nth(1).unwrap_or_else(|| "wzjs10.com".into());
    match acc.new_order(&domain, &[]) {
        Ok(mut order) => {
            println!("[probe] order created");
            match order.authorizations() {
                Ok(auths) => {
                    for a in &auths {
                        println!("[probe] auth domain: {}", a.domain_name());
                    }
                }
                Err(e) => println!("[probe] AUTHZ ERROR: {e:?}"),
            }
        }
        Err(e) => println!("[probe] ORDER ERROR: {e:?}"),
    }

    println!("[probe] --- app.log 中与 token/challenge 相关的调试行 ---");
    let log_path = dir.join("app.log");
    if let Ok(s) = std::fs::read_to_string(&log_path) {
        for line in s.lines().filter(|l| {
            l.contains("token") || l.contains("challenge") || l.contains("authz") || l.contains("DEBUG")
        }) {
            println!("{line}");
        }
    }
}
