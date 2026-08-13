//! 域名校验工具

use crate::error::{AppError, ErrorCode};

/// 校验域名（支持通配符 *.example.com，使用 IDNA 支持国际化域名）
pub fn validate_domain(input: &str) -> Result<String, AppError> {
    let d = input.trim().to_lowercase();
    if d.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidDomain, "域名不能为空"));
    }
    let (wildcard, host) = match d.strip_prefix("*.") {
        Some(rest) => (true, rest),
        None => (false, d.as_str()),
    };
    if wildcard && host.matches('.').count() < 1 {
        return Err(AppError::new(ErrorCode::InvalidDomain, "通配符域名至少需要一级子域，如 *.example.com"));
    }
    if host.contains('*') {
        return Err(AppError::new(ErrorCode::InvalidDomain, "域名中只能有一个 * 且必须位于开头"));
    }
    if host.contains('/') || host.contains(' ') || host.contains('@') {
        return Err(AppError::new(ErrorCode::InvalidDomain, "域名包含非法字符"));
    }
    // IDNA 校验
    match idna::domain_to_ascii(host) {
        Ok(ascii) if ascii.len() <= 253 => {
            let labels: Vec<&str> = ascii.split('.').collect();
            if labels.len() < 2 {
                return Err(AppError::new(ErrorCode::InvalidDomain, "域名至少包含两级，如 example.com"));
            }
            for l in &labels {
                if l.is_empty() || l.len() > 63 {
                    return Err(AppError::new(ErrorCode::InvalidDomain, "域名标签格式不正确"));
                }
                let valid = l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
                if !valid || l.starts_with('-') || l.ends_with('-') {
                    return Err(AppError::new(ErrorCode::InvalidDomain, "域名标签格式不正确"));
                }
            }
            let last = labels.last().unwrap();
            if last.chars().all(|c| c.is_ascii_digit()) {
                return Err(AppError::new(ErrorCode::InvalidDomain, "顶级域不能为纯数字（IP 不支持）"));
            }
            if wildcard {
                Ok(format!("*.{ascii}"))
            } else {
                Ok(ascii)
            }
        }
        Ok(_) => Err(AppError::new(ErrorCode::InvalidDomain, "域名过长")),
        Err(_) => Err(AppError::new(ErrorCode::InvalidDomain, "域名包含非法字符")),
    }
}

/// 从域名提取可查询的裸域（通配符 *.x.com → x.com；SAN 校验用）
pub fn bare_domain(domain: &str) -> String {
    domain.strip_prefix("*.").unwrap_or(domain).to_string()
}

/// 判断是否通配符
pub fn is_wildcard(domain: &str) -> bool {
    domain.starts_with("*.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_domains() {
        assert_eq!(validate_domain("example.com").unwrap(), "example.com");
        assert_eq!(validate_domain(" EXAMPLE.COM ").unwrap(), "example.com");
        assert_eq!(validate_domain("*.example.com").unwrap(), "*.example.com");
        assert_eq!(validate_domain("www.example.co.uk").unwrap(), "www.example.co.uk");
        assert_eq!(validate_domain("foo-bar.example.com").unwrap(), "foo-bar.example.com");
        // IDNA：中文域名转 punycode
        assert!(validate_domain("例子.测试").is_ok());
    }

    #[test]
    fn invalid_domains() {
        assert!(validate_domain("").is_err());
        assert!(validate_domain("example").is_err()); // 单级
        assert!(validate_domain("*.com").is_err()); // 通配符缺子域
        assert!(validate_domain("*.*.example.com").is_err()); // 多个 *
        assert!(validate_domain("192.168.1.1").is_err()); // 纯数字 TLD
        assert!(validate_domain("a b.com").is_err());
        assert!(validate_domain("foo/bar.com").is_err());
        assert!(validate_domain("-bad.example.com").is_err());
        assert!(validate_domain("bad-.example.com").is_err());
        assert!(validate_domain("exa mple.com").is_err());
    }

    #[test]
    fn wildcard_and_bare_domain() {
        assert!(is_wildcard("*.example.com"));
        assert!(!is_wildcard("example.com"));
        assert_eq!(bare_domain("*.example.com"), "example.com");
        assert_eq!(bare_domain("example.com"), "example.com");
    }
}
