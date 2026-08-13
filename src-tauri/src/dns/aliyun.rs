//! 阿里云 DNS（alidns）Provider
//!
//! RPC 风格 API：https://alidns.aliyuncs.com/
//! 签名：HMAC-SHA1（AccessKey 体系）

use async_trait::async_trait;
use base64::Engine;
use std::collections::BTreeMap;

use crate::error::{AppError, ErrorCode};
use crate::dns::DnsProvider;

const ENDPOINT: &str = "https://alidns.aliyuncs.com/";
const VERSION: &str = "2015-01-09";

pub struct AliyunProvider {
    access_key_id: String,
    access_key_secret: String,
}

impl AliyunProvider {
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self { access_key_id, access_key_secret }
    }

    fn sign(&self, params: &BTreeMap<String, String>) -> String {
        // canonicalized query: 按 key 排序，URL 编码 key=value，& 连接
        let sorted: Vec<(String, String)> = params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let canonical = sorted
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let string_to_sign = format!("GET&%2F&{}", percent_encode(&canonical));
        let key = format!("{}&", self.access_key_secret);
        let mac = hmac_sha1(key.as_bytes(), string_to_sign.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(mac)
    }

    async fn call(&self, action: &str, extra: &BTreeMap<String, String>) -> Result<serde_json::Value, AppError> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("Action".into(), action.into());
        params.insert("Version".into(), VERSION.into());
        params.insert("AccessKeyId".into(), self.access_key_id.clone());
        params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
        params.insert("SignatureVersion".into(), "1.0".into());
        params.insert("SignatureNonce".into(), uuid::Uuid::new_v4().to_string());
        params.insert("Timestamp".into(), utc_timestamp());
        params.insert("Format".into(), "JSON".into());
        params.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));

        let signature = self.sign(&params);
        params.insert("Signature".into(), signature);

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("{ENDPOINT}?{query}");

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| AppError::new(ErrorCode::DnsProviderApi, "无法连接阿里云 DNS 服务").detail(e.to_string()))?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.map_err(|e| {
            AppError::new(ErrorCode::DnsProviderApi, "解析阿里云响应失败").detail(e.to_string())
        })?;

        // 阿里云业务错误通常以 HTTP 200 返回，必须先检查 body.Code
        if let Some(code) = body["Code"].as_str().filter(|c| !c.is_empty()) {
            let msg = body["Message"].as_str().unwrap_or("未知错误").to_string();
            if code.contains("InvalidAccessKeyId") || code.contains("SignatureDoesNotMatch") {
                return Err(AppError::new(
                    ErrorCode::DnsProviderAuth,
                    "阿里云认证失败：AccessKey Secret 不正确（Secret 仅在创建密钥时显示一次，请重新创建密钥并完整复制 ID 和 Secret）",
                )
                .detail(msg));
            }
            // Forbidden：密钥有效，但账号缺少 DNS 权限（最常见原因：RAM 子账号未授权 AliyunDNSFullAccess）
            if code == "Forbidden" || code.contains("Forbidden") {
                return Err(AppError::new(
                    ErrorCode::DnsProviderAuth,
                    "阿里云密钥有效，但账号没有 DNS 管理权限（RAM 子账号需授权 AliyunDNSFullAccess）",
                )
                .detail(msg));
            }
            if code.contains("DomainNotExist") {
                return Err(AppError::new(ErrorCode::DnsTxtNotFound, "阿里云账户下找不到该域名").detail(msg));
            }
            return Err(AppError::new(ErrorCode::DnsProviderApi, format!("阿里云 API 错误: {code}")).detail(msg));
        }
        if !status.is_success() {
            return Err(AppError::new(ErrorCode::DnsProviderApi, format!("阿里云 HTTP 错误: {status}")).detail(body.to_string()));
        }
        Ok(body)
    }
}

#[async_trait]
impl DnsProvider for AliyunProvider {
    fn kind(&self) -> &'static str {
        "aliyun"
    }

    async fn add_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let rr = super::sub_of(record_name, &zone);
        let mut extra = BTreeMap::new();
        extra.insert("DomainName".into(), zone.clone());
        extra.insert("RR".into(), if rr == "@" { "@".into() } else { rr.clone() });
        extra.insert("Type".into(), "TXT".into());
        extra.insert("Value".into(), value.into());
        self.call("AddDomainRecord", &extra).await?;
        log::info!("aliyun: added TXT {rr}.{zone} = {value}");
        Ok(())
    }

    async fn remove_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let rr = super::sub_of(record_name, &zone);
        // 查找记录 ID
        let mut extra = BTreeMap::new();
        extra.insert("DomainName".into(), zone.clone());
        extra.insert("RRKeyWord".into(), rr.clone());
        extra.insert("TypeKeyWord".into(), "TXT".into());
        let body = self.call("DescribeDomainRecords", &extra).await?;
        let records = body["DomainRecords"]["Record"].as_array().cloned().unwrap_or_default();
        for rec in records {
            let rid = rec["RecordId"].as_str().unwrap_or("").to_string();
            let rec_value = rec["Value"].as_str().unwrap_or("").to_string();
            if rec_value.contains(value) {
                let mut del = BTreeMap::new();
                del.insert("RecordId".into(), rid);
                self.call("DeleteDomainRecord", &del).await?;
                log::info!("aliyun: deleted TXT {rr}.{zone}");
                // 不提前返回：删除所有匹配的重复记录
            }
        }
        Ok(())
    }

    async fn test(&self, _domain: &str) -> Result<(), AppError> {
        // 列出账户下所有域名（无需具体域名）
        let mut extra = BTreeMap::new();
        extra.insert("PageSize".into(), "1".into());
        self.call("DescribeDomains", &extra).await?;
        Ok(())
    }
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::sign::Signer;
    let pkey = PKey::hmac(key).unwrap();
    let mut signer = Signer::new(MessageDigest::sha1(), &pkey).unwrap();
    signer.update(data).unwrap();
    signer.sign_to_vec().unwrap()
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn utc_timestamp() -> String {
    use chrono::{SecondsFormat, Utc};
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// 阿里云官方文档的黄金签名向量（HMAC-SHA1）
    #[test]
    fn sign_matches_official_vector() {
        let p = AliyunProvider::new("testid".into(), "testsecret".into());
        let mut params = BTreeMap::new();
        params.insert("AccessKeyId".into(), "testid".into());
        params.insert("Action".into(), "DescribeRegions".into());
        params.insert("Format".into(), "XML".into());
        params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
        params.insert("SignatureNonce".into(), "3ee8c1b8-83d3-44af-a94f-4e0ad82fd6cf".into());
        params.insert("SignatureVersion".into(), "1.0".into());
        params.insert("TimeStamp".into(), "2016-02-23T12:46:24Z".into());
        params.insert("Version".into(), "2014-05-26".into());
        let sig = p.sign(&params);
        assert_eq!(sig, "CT9X0VtwR86fNWSnsc6v8YGOjuE=");
    }
}
