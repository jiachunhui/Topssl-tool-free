//! 证书落盘

use std::path::{Path, PathBuf};

use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use openssl::x509::X509;

use crate::error::{AppError, ErrorCode};

/// IIS 格式（PFX）默认密码
pub const IIS_PFX_PASSWORD: &str = "123456";

/// 证书文件布局：{certs_dir}/{domain}/fullchain.pem + privkey.pem
pub fn cert_dir(certs_root: &Path, domain: &str) -> PathBuf {
    // 通配符域名用下划线替代 *，避免路径非法字符
    let safe = domain.replace('*', "_");
    certs_root.join(safe)
}

/// 写临时 PEM 文件（POSIX 0600），返回临时文件路径
fn stage_pem(path: &Path, content: &str) -> Result<PathBuf, AppError> {
    let parent = path.parent().ok_or_else(|| AppError::new(ErrorCode::CertWrite, "证书路径无效"))?;
    std::fs::create_dir_all(parent).map_err(|e| {
        AppError::new(ErrorCode::CertWrite, "无法创建证书目录").detail(e.to_string())
    })?;

    let tmp = path.with_extension("pem.tmp");
    std::fs::write(&tmp, content).map_err(|e| {
        AppError::new(ErrorCode::CertWrite, "无法写入证书文件").detail(e.to_string())
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    Ok(tmp)
}

/// 写证书 bundle：两个文件先落临时文件，再连续 rename，
/// 尽量缩小"证书链与私钥不匹配"的崩溃窗口（续期时尤其重要）
pub fn write_bundle(
    certs_root: &Path,
    domain: &str,
    fullchain_pem: &str,
    private_key_pem: &str,
) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = cert_dir(certs_root, domain);
    let chain_path = dir.join("fullchain.pem");
    let key_path = dir.join("privkey.pem");
    let chain_tmp = stage_pem(&chain_path, fullchain_pem)?;
    let key_tmp = stage_pem(&key_path, private_key_pem)?;
    // 先提交私钥，再提交证书链
    std::fs::rename(&key_tmp, &key_path).map_err(|e| {
        AppError::new(ErrorCode::CertWrite, "无法保存私钥文件").detail(e.to_string())
    })?;
    std::fs::rename(&chain_tmp, &chain_path).map_err(|e| {
        AppError::new(ErrorCode::CertWrite, "无法保存证书文件").detail(e.to_string())
    })?;
    Ok((chain_path, key_path))
}

/// 删除证书目录（删除记录时调用）
pub fn remove_bundle(certs_root: &Path, domain: &str) -> std::io::Result<()> {
    let dir = cert_dir(certs_root, domain);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// 生成 IIS 可导入的 PFX（PKCS#12，含私钥 + 证书链）
pub fn write_pfx(
    dir: &Path,
    fullchain_pem: &str,
    private_key_pem: &str,
    password: &str,
) -> Result<PathBuf, AppError> {
    let pkey = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "私钥解析失败").detail(e.to_string()))?;
    let certs = X509::stack_from_pem(fullchain_pem.as_bytes())
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "证书链解析失败").detail(e.to_string()))?;
    let (leaf, cas) = certs
        .split_first()
        .ok_or_else(|| AppError::new(ErrorCode::CertWrite, "证书链为空"))?;

    let mut builder = Pkcs12::builder();
    builder.name("ssl-cert-desktop");
    builder.pkey(&pkey);
    builder.cert(leaf);
    if !cas.is_empty() {
        let mut ca_stack = openssl::stack::Stack::new()
            .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法构建证书链").detail(e.to_string()))?;
        for c in cas {
            ca_stack
                .push(c.clone())
                .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法构建证书链").detail(e.to_string()))?;
        }
        builder.ca(ca_stack);
    }
    let p12 = builder
        .build2(password)
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "PFX 生成失败").detail(e.to_string()))?;
    let der = p12
        .to_der()
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "PFX 序列化失败").detail(e.to_string()))?;

    let pfx_path = dir.join("cert.pfx");
    let tmp = pfx_path.with_extension("pfx.tmp");
    std::fs::write(&tmp, der)
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法写入 PFX 文件").detail(e.to_string()))?;
    std::fs::rename(&tmp, &pfx_path)
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法保存 PFX 文件").detail(e.to_string()))?;
    Ok(pfx_path)
}

/// 生成证书说明文本（记事本可读，UTF-8 带 BOM 保证中文正常显示）
pub fn write_readme(dir: &Path, domain: &str, pfx_password: &str) -> Result<PathBuf, AppError> {
    let content = format!(
        "SSL 证书说明\r\n\
         ================\r\n\
         \r\n\
         域名: {domain}\r\n\
         \r\n\
         文件清单:\r\n\
          1. fullchain.pem  证书链（PEM 格式，nginx / Apache 用）\r\n\
          2. privkey.pem    私钥（PEM 格式）\r\n\
          3. cert.pfx       IIS 格式证书（PKCS#12，密码: {pfx_password}）\r\n\
         \r\n\
         IIS 导入方法:\r\n\
          打开 IIS 管理器 → 服务器证书 → 导入 → 选择 cert.pfx →\r\n\
          输入密码 {pfx_password} → 确定，然后在站点绑定中选择该证书。\r\n\
         \r\n\
         注意: 证书每 90 天自动续期，续期后会同步更新上述文件；\r\n\
         在 IIS 中更新证书时请重新导入 cert.pfx。\r\n"
    );
    let path = dir.join("证书说明.txt");
    let mut bytes = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    bytes.extend_from_slice(content.as_bytes());
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, &bytes)
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法写入说明文件").detail(e.to_string()))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "无法保存说明文件").detail(e.to_string()))?;
    Ok(path)
}
