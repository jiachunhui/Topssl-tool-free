//! 部署包导出：把证书文件与各平台部署配置整理为一个文件夹，
//! 用户把整个文件夹拷贝到服务器，按说明配置即可完成部署。

use std::fs;
use std::path::{Path, PathBuf};

fn write(dir: &Path, name: &str, content: &str) -> Result<(), String> {
    fs::write(dir.join(name), content).map_err(|e| format!("写入 {name} 失败：{e}"))
}

/// 从证书链中提取第一个证书（叶子证书），供个别只需要单证书的服务使用
fn leaf_cert(chain: &str) -> &str {
    let begin = chain.find("-----BEGIN CERTIFICATE-----").unwrap_or(0);
    let end = chain[begin..]
        .find("-----END CERTIFICATE-----")
        .map(|i| begin + i + "-----END CERTIFICATE-----".len())
        .unwrap_or(chain.len());
    chain[begin..end].trim()
}

fn nginx_conf(domain: &str) -> String {
    format!(
        "# nginx 配置示例（{domain}）\n# 将证书目录拷贝到服务器后，按实际路径修改 ssl_certificate / ssl_certificate_key\nserver {{\n    listen 443 ssl;\n    server_name {domain};\n\n    ssl_certificate     /etc/nginx/ssl/{domain}/fullchain.pem;\n    ssl_certificate_key /etc/nginx/ssl/{domain}/privkey.pem;\n}}\n"
    )
}

fn apache_conf(domain: &str) -> String {
    format!(
        "# Apache 配置示例（{domain}）\n# 需要启用 mod_ssl；路径按实际部署位置修改\n<VirtualHost *:443>\n    ServerName {domain}\n    SSLEngine on\n    SSLCertificateFile    /etc/ssl/{domain}/fullchain.pem\n    SSLCertificateKeyFile /etc/ssl/{domain}/privkey.pem\n</VirtualHost>\n"
    )
}

fn iis_steps(domain: &str) -> String {
    format!(
        "IIS 部署步骤（{domain}）\n══════════════════════════\n\n方式一：ToSSL 一键部署（推荐）\n  1. 右键以管理员身份运行 ToSSL；\n  2. 在证书详情页点击「IIS 一键部署」，选择目标站点并确认；\n  3. 应用会自动导入证书并绑定 https（443 端口）。\n\n方式二：手动部署\n  1. 打开 IIS 管理器 → 服务器证书 → 导入 cert.pfx（密码：123456）；\n  2. 选择目标站点 → 绑定 → 添加 https 类型绑定，端口 443，主机名 {domain}；\n  3. 在「SSL 证书」下拉中选择刚导入的证书 → 确定；\n  4. 浏览器访问 https://{domain} 验证。\n\n说明：cert.pfx 仅用于 IIS；nginx / Apache 使用 fullchain.pem + privkey.pem。\n"
    )
}

fn readme(domain: &str) -> String {
    format!(
        "{domain} 证书部署包\n════════════════════════════════════════\n生成时间：{}\n\n文件清单\n  fullchain.pem       完整证书链（nginx / Apache 等所有服务使用）\n  privkey.pem         私钥（请妥善保管，勿公开；Linux 建议权限 600）\n  cert.pem            仅叶子证书（个别服务需要）\n  cert.pfx            IIS 导入用证书包（密码：123456）\n  nginx.conf.example  nginx 配置示例\n  apache.conf.example Apache 配置示例\n  IIS-部署步骤.txt     IIS 部署步骤\n\n部署方法\n  1. 将本目录拷贝到目标服务器（私钥建议通过 scp / 加密通道传输）；\n  2. 按服务器类型使用对应配置示例，把证书路径改为实际路径；\n  3. nginx 执行 nginx -s reload；Apache 执行 systemctl reload apache2；\n  4. Windows 服务器 + IIS：见「IIS-部署步骤.txt」，或在 ToSSL 中一键部署。\n\n注意：证书每 90 天到期。续期后可重新导出部署包替换服务器上的文件；\n建议在服务器上同时配置到期监控，避免证书过期导致 HTTPS 中断。\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    )
}

/// 导出部署包到 download_dir/<domain>-deploy-YYYYMMDD/，返回目录路径
pub fn export_deploy_package(
    domain: &str,
    chain_path: &str,
    key_path: &str,
    download_dir: &Path,
) -> Result<PathBuf, String> {
    let chain = fs::read_to_string(chain_path).map_err(|e| format!("读取证书链文件失败：{e}"))?;
    let key = fs::read_to_string(key_path).map_err(|e| format!("读取私钥文件失败：{e}"))?;

    let dir = download_dir.join(format!(
        "{domain}-deploy-{}",
        chrono::Utc::now().format("%Y%m%d")
    ));
    fs::create_dir_all(&dir).map_err(|e| format!("创建部署包目录失败：{e}"))?;

    write(&dir, "fullchain.pem", chain.trim())?;
    write(&dir, "privkey.pem", key.trim())?;
    write(&dir, "cert.pem", leaf_cert(&chain))?;

    // 证书目录中的 cert.pfx（IIS 导入用）一并复制
    if let Some(parent) = Path::new(chain_path).parent() {
        let pfx = parent.join("cert.pfx");
        if pfx.exists() {
            let _ = fs::copy(&pfx, dir.join("cert.pfx"));
        }
    }

    write(&dir, "nginx.conf.example", &nginx_conf(domain))?;
    write(&dir, "apache.conf.example", &apache_conf(domain))?;
    write(&dir, "IIS-部署步骤.txt", &iis_steps(domain))?;
    write(&dir, "部署说明.txt", &readme(domain))?;

    Ok(dir)
}
