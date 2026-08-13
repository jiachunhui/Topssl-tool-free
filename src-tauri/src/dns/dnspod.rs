//! DNSPod（腾讯云）Provider
//!
//! 使用 DNSPod 旧版开放 API（dnsapi.cn），login_token 方式最简单：
//! https://docs.dnspod.cn/api/old-api/

use async_trait::async_trait;
use std::collections::BTreeMap;

use crate::dns::DnsProvider;
use crate::error::{AppError, ErrorCode};

const API: &str = "https://dnsapi.cn";

pub struct DnspodProvider {
    token_id: String,
    login_token: String, // "id,token" 或旧格式 token 本身
}

impl DnspodProvider {
    pub fn new(token_id: String, login_token: String) -> Self {
        Self { token_id, login_token }
    }

    fn auth_params(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        let token = if self.login_token.contains(',') {
            self.login_token.clone()
        } else if !self.token_id.is_empty() {
            format!("{},{}", self.token_id, self.login_token)
        } else {
            self.login_token.clone()
        };
        m.insert("login_token".into(), token);
        m.insert("format".into(), "json".into());
        m
    }

    async fn call(&self, action: &str, extra: &BTreeMap<String, String>) -> Result<serde_json::Value, AppError> {
        let mut form: BTreeMap<String, String> = self.auth_params();
        form.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
        // DNSPod 要求值做 URL 编码（值内可能含特殊字符）
        let body = form
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{API}/{action}"))
            .header("User-Agent", "ssl-cert-desktop/0.1 (jiachunhui)")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| AppError::new(ErrorCode::DnsProviderApi, "无法连接 DNSPod 服务").detail(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::new(ErrorCode::DnsProviderApi, "解析 DNSPod 响应失败").detail(e.to_string()))?;

        let status = json["status"]["code"].as_str().unwrap_or("").to_string();
        let msg = json["status"]["message"].as_str().unwrap_or("未知错误").to_string();
        // 成功码精确为 "1"；错误码 "10"/"12" 等以 1 开头，不能用前缀判断
        if status != "1" {
            // 认证类错误
            if status.starts_with("7") || status.starts_with("8") || status.starts_with("6") {
                return Err(AppError::new(ErrorCode::DnsProviderAuth, "DNSPod 认证失败").detail(msg));
            }
            if status == "10" || status == "12" {
                return Err(AppError::new(ErrorCode::DnsTxtNotFound, "DNSPod 账户下找不到该域名").detail(msg));
            }
            return Err(AppError::new(ErrorCode::DnsProviderApi, format!("DNSPod API 错误 ({status})")).detail(msg));
        }
        Ok(json)
    }
}

#[async_trait]
impl DnsProvider for DnspodProvider {
    fn kind(&self) -> &'static str {
        "dnspod"
    }

    async fn add_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let sub = super::sub_of(record_name, &zone);
        let mut extra = BTreeMap::new();
        extra.insert("domain".into(), zone.clone());
        extra.insert("sub_domain".into(), if sub == "@" { "@".into() } else { sub.clone() });
        extra.insert("record_type".into(), "TXT".into());
        extra.insert("record_line".into(), "默认".into());
        extra.insert("value".into(), value.into());
        self.call("Record.Create", &extra).await?;
        log::info!("dnspod: added TXT {sub}.{zone}");
        Ok(())
    }

    async fn remove_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let sub = super::sub_of(record_name, &zone);
        let mut extra = BTreeMap::new();
        extra.insert("domain".into(), zone.clone());
        extra.insert("sub_domain".into(), if sub == "@" { "@".into() } else { sub.clone() });
        let body = self.call("Record.List", &extra).await?;
        let records = body["records"].as_array().cloned().unwrap_or_default();
        for rec in records {
            let rid = rec["id"].as_str().unwrap_or("").to_string();
            let rec_type = rec["type"].as_str().unwrap_or("").to_string();
            let rec_value = rec["value"].as_str().unwrap_or("").to_string();
            if rec_type == "TXT" && rec_value.contains(value) {
                let mut del = BTreeMap::new();
                del.insert("domain".into(), zone.clone());
                del.insert("record_id".into(), rid);
                self.call("Record.Remove", &del).await?;
                log::info!("dnspod: removed TXT {sub}.{zone}");
                // 不提前返回：删除所有匹配的重复记录
            }
        }
        Ok(())
    }

    async fn test(&self, _domain: &str) -> Result<(), AppError> {
        // 列出账户下域名
        let extra = BTreeMap::new();
        self.call("Domain.List", &extra).await?;
        Ok(())
    }
}

fn urlencode(s: &str) -> String {
    // DNSPod 官方要求的编码方式
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
