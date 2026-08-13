// 诊断工具：对现有证书目录生成 PFX 并回读校验（排查用，可随时删除）
// 用法: cargo run --example pfx_check -- <证书目录>
fn main() {
    let dir = std::env::args().nth(1).expect("usage: pfx_check <cert_dir>");
    let chain = std::fs::read_to_string(format!("{dir}/fullchain.pem")).unwrap_or_default();
    let key = std::fs::read_to_string(format!("{dir}/privkey.pem")).unwrap_or_default();
    if chain.is_empty() || key.is_empty() {
        println!("[pfx] 证书文件不存在: {dir}");
        return;
    }
    match ssl_cert_desktop_lib::cert::store::write_pfx(
        std::path::Path::new(&dir),
        &chain,
        &key,
        ssl_cert_desktop_lib::cert::store::IIS_PFX_PASSWORD,
    ) {
        Ok(p) => {
            println!("[pfx] 生成成功: {}", p.display());
            // 回读校验：能解析回来且密码正确
            let der = std::fs::read(&p).unwrap();
            match openssl::pkcs12::Pkcs12::from_der(&der)
                .and_then(|p12| p12.parse2(ssl_cert_desktop_lib::cert::store::IIS_PFX_PASSWORD))
            {
                Ok(p) => println!("[pfx] 回读校验通过，含证书 {} 张", p.cert.as_ref().map(|_| 1).unwrap_or(0)),
                Err(e) => println!("[pfx] 回读校验失败: {e}"),
            }
        }
        Err(e) => println!("[pfx] 生成失败: {e}"),
    }
    match ssl_cert_desktop_lib::cert::store::write_readme(
        std::path::Path::new(&dir),
        "www.wzjs100.com",
        ssl_cert_desktop_lib::cert::store::IIS_PFX_PASSWORD,
    ) {
        Ok(p) => println!("[pfx] 说明文件生成成功: {}", p.display()),
        Err(e) => println!("[pfx] 说明文件生成失败: {e}"),
    }
}
