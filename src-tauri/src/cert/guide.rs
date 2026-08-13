//! 证书使用指引生成

/// 生成本机 HTTPS 服务的引用指引（nginx / Apache / 其他）
pub fn generate_guide(domain: &str, chain_path: &str, key_path: &str, platform: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "证书文件（{domain}）：\n  证书链: {chain_path}\n  私钥: {key_path}\n\n"
    ));

    s.push_str("▎Nginx 配置示例\n\n");
    s.push_str(&format!(
        "server {{\n    listen 443 ssl;\n    server_name {domain};\n\n    ssl_certificate     {chain_path};\n    ssl_certificate_key {key_path};\n\n    location / {{\n        proxy_pass http://127.0.0.1:8080;\n    }}\n}}\n\n"
    ));

    s.push_str("▎Apache 配置示例\n\n");
    s.push_str(&format!(
        "<VirtualHost *:443>\n    ServerName {domain}\n    SSLEngine on\n    SSLCertificateFile {chain_path}\n    SSLCertificateKeyFile {key_path}\n</VirtualHost>\n\n"
    ));

    s.push_str("▎其他本地服务\n\n");
    s.push_str(&format!(
        "· Node.js (https): 将 `cert` 指向 {chain_path}，`key` 指向 {key_path}\n"
    ));
    s.push_str(&format!(
        "· Python (http.server): python -m http.server 443 --certfile \"{chain_path}\" --keyfile \"{key_path}\"\n"
    ));
    s.push_str(&format!(
        "· IIS: 在 IIS 管理器 → 服务器证书 → 导入 cert.pfx（密码 123456），\n\
         证书目录下有「证书说明.txt」可查看详细步骤\n"
    ));
    s.push_str("· 任何需要 PEM 的服务：直接引用上述两个文件路径即可\n\n");

    s.push_str("注意：证书每 90 天自动续期，程序会直接更新上述文件，服务重启后生效。\n");

    if platform == "windows" {
        s.push_str("\n提示：若 nginx/apache 以服务方式运行，更新证书后请执行 `nginx -s reload` 或重启服务。\n");
    }
    s
}
