//! ACME 申请流程模型：阶段 / 任务状态 / 进度事件

use serde::Serialize;

/// 申请流水线阶段
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum IssueStage {
    InputValidated,
    DirectoryReady,
    AccountRegistered,
    OrderCreated,
    AuthorizationsFetched,
    ChallengePrepared,
    ChallengeServed,
    ValidationInProgress,
    Validated,
    CsrSubmitted,
    CertReady,
    CertDownloaded,
    CertInstalled,
    RecordUpdated,
    Completed,
}

impl IssueStage {
    pub fn index(&self) -> usize {
        match self {
            IssueStage::InputValidated => 0,
            IssueStage::DirectoryReady => 1,
            IssueStage::AccountRegistered => 2,
            IssueStage::OrderCreated => 3,
            IssueStage::AuthorizationsFetched => 4,
            IssueStage::ChallengePrepared => 5,
            IssueStage::ChallengeServed => 6,
            IssueStage::ValidationInProgress => 7,
            IssueStage::Validated => 8,
            IssueStage::CsrSubmitted => 9,
            IssueStage::CertReady => 10,
            IssueStage::CertDownloaded => 11,
            IssueStage::CertInstalled => 12,
            IssueStage::RecordUpdated => 13,
            IssueStage::Completed => 14,
        }
    }

    /// 对应前端展示文案
    pub fn label(&self) -> &'static str {
        match self {
            IssueStage::InputValidated => "校验域名",
            IssueStage::DirectoryReady => "连接 ACME 服务",
            IssueStage::AccountRegistered => "注册账户",
            IssueStage::OrderCreated => "创建订单",
            IssueStage::AuthorizationsFetched => "获取授权",
            IssueStage::ChallengePrepared => "准备验证",
            IssueStage::ChallengeServed => "响应验证",
            IssueStage::ValidationInProgress => "等待验证结果",
            IssueStage::Validated => "验证通过",
            IssueStage::CsrSubmitted => "提交签发请求",
            IssueStage::CertReady => "证书签发完成",
            IssueStage::CertDownloaded => "下载证书",
            IssueStage::CertInstalled => "安装证书",
            IssueStage::RecordUpdated => "更新记录",
            IssueStage::Completed => "完成",
        }
    }
}

/// 任务状态（对前端；字段用蛇形命名序列化，与前端 types.ts 一致）
#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub job_id: String,
    pub state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<IssueStage>,
    pub percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_id: Option<i64>,
    /// 任务目标域名（续期去重 / 前端悬浮条展示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Pending,
    Running,
    Failed,
    Canceled,
    Completed,
}

/// 进度事件 payload（Rust → 前端；蛇形命名与前端 types.ts 一致）
#[derive(Debug, Clone, Serialize)]
pub struct JobProgress {
    pub job_id: String,
    pub stage: IssueStage,
    pub percent: u8,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 申请任务配置（由 start_issue command 传入；蛇形命名与前端 IssueRequest 一致）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct IssueRequest {
    pub domain: String,
    #[serde(default)]
    pub alt_names: Vec<String>,
    pub challenge_type: String, // "http01" | "dns01"
    #[serde(default)]
    pub provider_id: Option<i64>,
    /// DNS-01 手动模式：用户自行去 DNS 控制台添加 TXT 记录（无需 API）
    #[serde(default)]
    pub dns_manual: bool,
    #[serde(default = "default_directory")]
    pub directory: String, // "staging" | "production"
    pub contact_email: String,
}

fn default_directory() -> String {
    "staging".to_string()
}

/// 订单结果（证书 bundle）
#[derive(Debug, Clone)]
pub struct CertBundle {
    pub fullchain_pem: String,
    pub private_key_pem: String,
    pub issuer: String,
    pub order_url: Option<String>,
    pub not_after: String, // RFC3339
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归：字段必须蛇形序列化（与前端 types.ts 一致）。
    /// 曾因 rename_all="camelCase" 导致前端读到 undefined（设置页无值、向导崩溃）
    #[test]
    fn job_progress_serializes_snake_case() {
        let p = JobProgress {
            job_id: "j1".into(),
            stage: IssueStage::Validated,
            percent: 50,
            message: "ok".into(),
            detail: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert!(v.get("job_id").is_some());
        assert!(v.get("jobId").is_none());
    }

    #[test]
    fn issue_request_deserializes_snake_case() {
        let json = r#"{
            "domain": "example.com",
            "alt_names": ["www.example.com"],
            "challenge_type": "dns01",
            "provider_id": null,
            "dns_manual": true,
            "directory": "staging",
            "contact_email": "a@b.com"
        }"#;
        let req: IssueRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.challenge_type, "dns01");
        assert_eq!(req.alt_names, vec!["www.example.com".to_string()]);
        assert!(req.dns_manual);
    }
}
