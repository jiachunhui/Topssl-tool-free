//! DNS Provider 抽象层

pub mod aliyun;
pub mod cloudflare;
pub mod dnspod;

use async_trait::async_trait;
use std::time::Duration;

use crate::error::{AppError, ErrorCode};
use crate::secret::keyring::SecretStore;
use crate::storage::providers::ProviderRow;

#[async_trait]
pub trait DnsProvider: Send + Sync {
    fn kind(&self) -> &'static str;
    /// 添加 TXT 记录
    async fn add_txt(&self, domain: &str, record_name: &str, value: &str) -> Result<(), AppError>;
    /// 删除 TXT 记录
    async fn remove_txt(&self, domain: &str, record_name: &str, value: &str) -> Result<(), AppError>;
    /// 只读探测：验证凭证有效（不依赖具体域名）
    async fn test(&self, domain: &str) -> Result<(), AppError>;
}

/// 根据数据库记录构建 Provider 实例（机密从 keyring 读取）
pub fn build_provider(row: &ProviderRow, secrets: &SecretStore) -> Result<Box<dyn DnsProvider>, AppError> {
    let config: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&row.config_json).unwrap_or_default();

    let get = |k: &str| -> Option<String> {
        config.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    // 从 keyring 读取机密（合并到 config 视图）
    let secret_json = secrets.load(&row.secret_ref)?.unwrap_or_default();
    let secret_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&secret_json).unwrap_or_default();
    let get_secret = |k: &str| -> Option<String> {
        secret_map.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    match row.kind {
        crate::storage::providers::ProviderKind::Aliyun => {
            let secret = get_secret("access_key_secret").or_else(|| get("access_key_secret")).unwrap_or_default();
            if secret.trim().is_empty() {
                return Err(AppError::new(
                    ErrorCode::DnsProviderAuth,
                    "密钥存储中未找到阿里云 Secret（可能之前保存失败），请编辑该服务商并重新填写 Secret 后保存",
                ));
            }
            let p = aliyun::AliyunProvider::new(
                get_secret("access_key_id").or_else(|| get("access_key_id")).unwrap_or_default(),
                secret,
            );
            Ok(Box::new(p))
        }
        crate::storage::providers::ProviderKind::Dnspod => {
            let secret = get_secret("login_token").or_else(|| get("login_token")).unwrap_or_default();
            if secret.trim().is_empty() {
                return Err(AppError::new(
                    ErrorCode::DnsProviderAuth,
                    "密钥存储中未找到 DNSPod Token（可能之前保存失败），请编辑该服务商并重新填写后保存",
                ));
            }
            let p = dnspod::DnspodProvider::new(get("token_id").unwrap_or_default(), secret);
            Ok(Box::new(p))
        }
        crate::storage::providers::ProviderKind::Cloudflare => {
            let secret = get_secret("api_token").or_else(|| get("api_token")).unwrap_or_default();
            if secret.trim().is_empty() {
                return Err(AppError::new(
                    ErrorCode::DnsProviderAuth,
                    "密钥存储中未找到 Cloudflare Token（可能之前保存失败），请编辑该服务商并重新填写后保存",
                ));
            }
            let p = cloudflare::CloudflareProvider::new(secret);
            Ok(Box::new(p))
        }
    }
}

/// 轮询等待 TXT 记录生效
/// 使用 Cloudflare 公共 DNS（1.1.1.1）绕开本机/ISP 解析器缓存：
/// 真实案例中，同一记录名下先后添加两条 TXT 时，本机解析器缓存旧答案
/// （TTL 最长 600s），导致后添加的记录已生效却检查不到、等待超时。
/// LE 自身的验证直连权威 DNS 不受缓存影响，我们的检查也应与之保持一致。
pub async fn wait_propagation(record_name: &str, value: &str, timeout: Duration) -> Result<(), AppError> {
    use hickory_resolver::config::ResolverConfig;
    use hickory_resolver::name_server::TokioConnectionProvider;
    use hickory_resolver::TokioResolver;

    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::cloudflare(),
        TokioConnectionProvider::default(),
    )
    .build();

    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::DnsPropagationTimeout,
                "TXT 记录传播超时（已等待 120 秒）",
            ));
        }
        match resolver.txt_lookup(record_name).await {
            Ok(lookup) => {
                let found = lookup.iter().any(|r| r.to_string().contains(value.trim_matches('"')));
                if found {
                    return Ok(());
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// 从完整记录名提取所属区域（如 _acme-challenge.www.example.co.uk → example.co.uk）
pub fn zone_of(record_name: &str) -> String {
    let name = record_name.trim_end_matches('.').to_lowercase();
    // 优先用公共后缀列表（PSL）求注册域，正确处理 co.uk / com.cn 等多级后缀
    if let Some(registrable) = psl::domain_str(&name) {
        if !registrable.is_empty() {
            return registrable.to_string();
        }
    }
    // 兜底：取最后两级
    let labels: Vec<&str> = name.split('.').collect();
    if labels.len() >= 2 {
        labels[labels.len() - 2..].join(".")
    } else {
        name.to_string()
    }
}

/// 从记录名去除区域前缀（_acme-challenge.www.example.com / example.com → _acme-challenge.www）
pub fn sub_of(record_name: &str, zone: &str) -> String {
    let name = record_name.trim_end_matches('.');
    if let Some(rest) = name.strip_suffix(zone) {
        rest.trim_end_matches('.').to_string()
    } else {
        "@".to_string()
    }
}

pub fn dns_provider_error(e: impl std::fmt::Display, msg: &str) -> AppError {
    log::error!("DNS provider error: {e}");
    AppError::new(ErrorCode::DnsProviderApi, msg).detail(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_of_simple_tld() {
        assert_eq!(zone_of("_acme-challenge.example.com"), "example.com");
        assert_eq!(zone_of("_acme-challenge.www.example.com"), "example.com");
        assert_eq!(zone_of("_acme-challenge.example.com."), "example.com");
    }

    #[test]
    fn zone_of_multi_label_tld() {
        // PSL 多级后缀：旧"最后两级"启发式会得出错误的 co.uk / com.cn
        assert_eq!(zone_of("_acme-challenge.www.example.co.uk"), "example.co.uk");
        assert_eq!(zone_of("_acme-challenge.www.example.com.cn"), "example.com.cn");
        assert_eq!(zone_of("_acme-challenge.foo.github.io"), "foo.github.io");
    }

    #[test]
    fn sub_of_strips_zone() {
        assert_eq!(sub_of("_acme-challenge.www.example.com", "example.com"), "_acme-challenge.www");
        assert_eq!(sub_of("_acme-challenge.example.com", "example.com"), "_acme-challenge");
    }
}

#[cfg(test)]
mod extra_tests {
    /// 回归：传播检查使用的公共 DNS（Cloudflare）解析器可正常构建
    /// （绕开本机/ISP 缓存，避免"同名多条 TXT 后添加的记录检查不到"）
    #[test]
    fn public_resolver_builds() {
        use hickory_resolver::config::ResolverConfig;
        use hickory_resolver::name_server::TokioConnectionProvider;
        use hickory_resolver::TokioResolver;
        let _resolver = TokioResolver::builder_with_config(
            ResolverConfig::cloudflare(),
            TokioConnectionProvider::default(),
        )
        .build();
    }
}
