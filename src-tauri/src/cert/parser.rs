//! 证书解析：从 PEM 提取 issuer / not_after / SAN

use crate::error::{AppError, ErrorCode};

#[derive(Debug, Clone)]
pub struct ParsedCert {
    pub issuer: String,
    pub not_after: String, // RFC3339
    pub san: Vec<String>,
}

/// 解析证书链 PEM（取第一张叶子证书）
pub fn parse_bundle(fullchain_pem: &str) -> Result<ParsedCert, AppError> {
    use openssl::x509::X509;
    let cert = X509::from_pem(fullchain_pem.as_bytes())
        .map_err(|e| AppError::new(ErrorCode::CertWrite, "证书解析失败").detail(e.to_string()))?;

    let issuer = cert
        .issuer_name()
        .entries()
        .filter_map(|e| e.data().to_string().ok())
        .collect::<Vec<_>>()
        .join(", ");

    let not_after = cert.not_after().to_string(); // e.g. "Aug 10 12:00:00 2026 GMT"
    let not_after_rfc3339 = parse_asn1_time(&not_after);

    let san = subject_alt_names(&cert);

    Ok(ParsedCert { issuer, not_after: not_after_rfc3339, san })
}

fn parse_asn1_time(s: &str) -> String {
    // "Aug 10 12:00:00 2026 GMT" → RFC3339
    use chrono::NaiveDateTime;
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%b %e %H:%M:%S %Y GMT") {
        return chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc).to_rfc3339();
    }
    s.to_string()
}

fn subject_alt_names(cert: &openssl::x509::X509) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(names) = cert.subject_alt_names() {
        for n in names.iter() {
            if let Some(d) = n.dnsname() {
                out.push(d.to_string());
            }
        }
    }
    out
}

/// 剩余天数
pub fn days_remaining(expires_at: &str) -> i64 {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(expires_at) else {
        return 0;
    };
    let now = chrono::Utc::now();
    (dt.with_timezone(&chrono::Utc) - now).num_days()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asn1_time_converted_to_rfc3339() {
        assert_eq!(parse_asn1_time("Aug 10 12:00:00 2026 GMT"), "2026-08-10T12:00:00+00:00");
        assert_eq!(parse_asn1_time("Feb  5 08:30:00 2027 GMT"), "2027-02-05T08:30:00+00:00");
    }

    #[test]
    fn unknown_format_passthrough() {
        assert_eq!(parse_asn1_time("not-a-time"), "not-a-time");
    }
}
