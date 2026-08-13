//! 统一错误类型与错误码（与前端 src/lib/errors.ts 一致）

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidDomain,
    AcmeConnection,
    AcmeAccount,
    AcmeRateLimit,
    OrderCreate,
    ChallengeUnsupported,
    Http01PortBusy,
    Http01Unreachable,
    Http01Privilege,
    DnsProviderAuth,
    DnsProviderApi,
    DnsTxtNotFound,
    DnsPropagationTimeout,
    ValidationFailed,
    FinalizeFailed,
    CertDownload,
    CertWrite,
    DuplicateCert,
    Canceled,
    CoolDown,
    InvalidSetting,
    Db,
    Internal,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidDomain => "ERR_INVALID_DOMAIN",
            ErrorCode::AcmeConnection => "ERR_ACME_CONNECTION",
            ErrorCode::AcmeAccount => "ERR_ACME_ACCOUNT",
            ErrorCode::AcmeRateLimit => "ERR_ACME_RATE_LIMIT",
            ErrorCode::OrderCreate => "ERR_ORDER_CREATE",
            ErrorCode::ChallengeUnsupported => "ERR_CHALLENGE_UNSUPPORTED",
            ErrorCode::Http01PortBusy => "ERR_HTTP01_PORT_BUSY",
            ErrorCode::Http01Unreachable => "ERR_HTTP01_UNREACHABLE",
            ErrorCode::Http01Privilege => "ERR_HTTP01_PRIVILEGE",
            ErrorCode::DnsProviderAuth => "ERR_DNS_PROVIDER_AUTH",
            ErrorCode::DnsProviderApi => "ERR_DNS_PROVIDER_API",
            ErrorCode::DnsTxtNotFound => "ERR_DNS_TXT_NOT_FOUND",
            ErrorCode::DnsPropagationTimeout => "ERR_DNS_PROPAGATION_TIMEOUT",
            ErrorCode::ValidationFailed => "ERR_VALIDATION_FAILED",
            ErrorCode::FinalizeFailed => "ERR_FINALIZE_FAILED",
            ErrorCode::CertDownload => "ERR_CERT_DOWNLOAD",
            ErrorCode::CertWrite => "ERR_CERT_WRITE",
            ErrorCode::DuplicateCert => "ERR_DUPLICATE_CERT",
            ErrorCode::Canceled => "ERR_CANCELED",
            ErrorCode::CoolDown => "ERR_COOL_DOWN",
            ErrorCode::InvalidSetting => "ERR_INVALID_SETTING",
            ErrorCode::Db => "ERR_DB",
            ErrorCode::Internal => "ERR_INTERNAL",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), detail: None }
    }
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    pub fn internal(e: impl std::fmt::Display) -> Self {
        log::error!("Internal error: {e}");
        Self::new(ErrorCode::Internal, "内部错误").detail(e.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        log::error!("DB error: {e}");
        AppError::new(ErrorCode::Db, "数据库操作失败").detail(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        log::error!("JSON error: {e}");
        AppError::new(ErrorCode::Internal, "数据解析失败").detail(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::new(ErrorCode::Internal, "IO 操作失败").detail(e.to_string())
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        log::warn!("Keyring error: {e}");
        AppError::new(ErrorCode::Internal, "密钥存储访问失败").detail(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
