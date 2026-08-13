//! Cloudflare DNS Provider
//!
//! API: https://api.cloudflare.com/client/v4/
//! 认证：Bearer Token（需 Zone:DNS:Edit 权限）

use async_trait::async_trait;

use crate::dns::DnsProvider;
use crate::error::{AppError, ErrorCode};

const API: &str = "https://api.cloudflare.com/client/v4";

pub struct CloudflareProvider {
    api_token: String,
}

impl CloudflareProvider {
    pub fn new(api_token: String) -> Self {
        Self { api_token }
    }

    async fn call(&self, method: &str, path: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, AppError> {
        let client = reqwest::Client::new();
        let url = format!("{API}{path}");
        let mut req = client.request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), &url).bearer_auth(&self.api_token);
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| AppError::new(ErrorCode::DnsProviderApi, "无法连接 Cloudflare 服务").detail(e.to_string()))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::new(ErrorCode::DnsProviderApi, "解析 Cloudflare 响应失败").detail(e.to_string()))?;

        if json["success"] != true {
            let msgs: Vec<String> = json["errors"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e["message"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let joined = msgs.join("; ");
            let code = json["errors"][0]["code"].as_i64().unwrap_or(0);
            // 认证错误 1000/9109/9109/9119
            if code == 1000 || code == 9109 || code == 9119 || joined.contains("auth") {
                return Err(AppError::new(ErrorCode::DnsProviderAuth, "Cloudflare 认证失败").detail(joined));
            }
            return Err(AppError::new(ErrorCode::DnsProviderApi, "Cloudflare API 错误").detail(joined));
        }
        Ok(json)
    }

    async fn zone_id(&self, zone: &str) -> Result<String, AppError> {
        let path = format!("/zones?name={}", urlencode(zone));
        let json = self.call("GET", &path, None).await?;
        let zones = json["result"].as_array().cloned().unwrap_or_default();
        zones
            .first()
            .and_then(|z| z["id"].as_str().map(String::from))
            .ok_or_else(|| AppError::new(ErrorCode::DnsTxtNotFound, "Cloudflare 账户下找不到该域名"))
    }
}

#[async_trait]
impl DnsProvider for CloudflareProvider {
    fn kind(&self) -> &'static str {
        "cloudflare"
    }

    async fn add_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let zone_id = self.zone_id(&zone).await?;
        let body = serde_json::json!({
            "type": "TXT",
            "name": record_name.trim_end_matches('.'),
            "content": value,
            "ttl": 120
        });
        self.call("POST", &format!("/zones/{zone_id}/dns_records"), Some(body)).await?;
        log::info!("cloudflare: added TXT {record_name}");
        Ok(())
    }

    async fn remove_txt(&self, _domain: &str, record_name: &str, value: &str) -> Result<(), AppError> {
        let zone = super::zone_of(record_name);
        let zone_id = self.zone_id(&zone).await?;
        let path = format!("/zones/{zone_id}/dns_records?type=TXT&name={}", urlencode(record_name.trim_end_matches('.')));
        let json = self.call("GET", &path, None).await?;
        let records = json["result"].as_array().cloned().unwrap_or_default();
        for rec in records {
            let rid = rec["id"].as_str().unwrap_or("").to_string();
            let content = rec["content"].as_str().unwrap_or("").to_string();
            if content.contains(value) {
                self.call("DELETE", &format!("/zones/{zone_id}/dns_records/{rid}"), None).await?;
                log::info!("cloudflare: removed TXT {record_name}");
                // 不提前返回：删除所有匹配的重复记录
            }
        }
        Ok(())
    }

    async fn test(&self, _domain: &str) -> Result<(), AppError> {
        // 仅验证凭证有效：列出账户下 zone（不依赖具体域名）
        self.call("GET", "/zones?per_page=1", None).await?;
        Ok(())
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
