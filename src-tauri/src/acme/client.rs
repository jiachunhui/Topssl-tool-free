//! acme-micro 同步封装：账户注册/加载、订单创建、挑战、签发、下载
//!
//! 注意：acme-micro 是同步阻塞 API，且中间对象（NewOrder/Challenge）持有内部状态，
//! 因此整个订单生命周期必须在同一个 spawn_blocking 闭包内执行（见 flow.rs）。
//! acme-micro 0.14 仅 re-export 了 Directory/Account/Certificate 等类型，
//! NewOrder/Auth/CsrOrder 等类型不可命名，直接由方法签名推断。

use std::time::Duration;

use acme_micro::{Directory, DirectoryUrl, Error as AcmeError};

use crate::error::{AppError, ErrorCode};

pub type AcmeResult<T> = Result<T, AppError>;

/// 目录 URL
pub fn directory_url(directory: &str) -> DirectoryUrl<'static> {
    if directory == "production" {
        DirectoryUrl::LetsEncrypt
    } else {
        DirectoryUrl::LetsEncryptStaging
    }
}

/// 创建 Directory
pub fn connect(directory: &str) -> AcmeResult<Directory> {
    Directory::from_url(directory_url(directory))
        .map_err(|e| map_acme_error(e, "无法连接 ACME 服务"))
}

/// 注册新账户，返回账户私钥 PEM
pub fn register_account(dir: &Directory, contact_email: &str) -> AcmeResult<String> {
    let contacts = vec![format!("mailto:{contact_email}")];
    let acc = dir
        .register_account(contacts)
        .map_err(|e| map_acme_error(e, "账户注册失败"))?;
    acc.acme_private_key_pem()
        .map_err(|e| map_acme_error(e, "无法导出账户密钥"))
}

/// 用已有账户私钥加载账户
pub fn load_account(dir: &Directory, account_key_pem: &str, contact_email: &str) -> AcmeResult<acme_micro::Account> {
    let contacts = vec![format!("mailto:{contact_email}")];
    dir.load_account(account_key_pem, contacts)
        .map_err(|e| map_acme_error(e, "加载账户失败"))
}

/// 生成证书私钥，类型为 openssl PKey（acme-micro finalize_pkey 需要）
/// key_type: "rsa"（兼容性最好，默认）| "ecc"（P-384，更快更安全）
pub type CertKey = openssl::pkey::PKey<openssl::pkey::Private>;

pub fn create_key(key_type: &str) -> AcmeResult<CertKey> {
    if key_type == "ecc" {
        acme_micro::create_p384_key().map_err(|e| map_acme_error(e, "生成密钥失败"))
    } else {
        acme_micro::create_rsa_key(2048).map_err(|e| map_acme_error(e, "生成密钥失败"))
    }
}

/// ACME 错误 → 错误码映射
pub fn map_acme_error(e: AcmeError, default_msg: &str) -> AppError {
    let msg = e.to_string();
    log::error!("ACME error: {msg}");
    if msg.contains("too many certificates") || msg.contains("rateLimit") || msg.contains("Rate limit") {
        return AppError::new(ErrorCode::AcmeRateLimit, "触发了 Let's Encrypt 速率限制").detail(msg);
    }
    if msg.contains("connection") || msg.contains("timed out") || msg.contains("IO error") || msg.contains("Curl") {
        return AppError::new(ErrorCode::AcmeConnection, "无法连接 ACME 服务").detail(msg);
    }
    if msg.contains("validation") || msg.contains("unauthorized") || msg.contains("dns problem") {
        return AppError::new(ErrorCode::ValidationFailed, "域名所有权验证失败").detail(msg);
    }
    AppError::new(ErrorCode::OrderCreate, default_msg).detail(msg)
}

pub fn acme_wait() -> Duration {
    Duration::from_millis(5000)
}
