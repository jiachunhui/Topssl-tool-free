// 与 Rust 端 serde 序列化类型一一对应

/** 应用信息 */
export interface AppInfo {
  version: string
  platform: string
  arch: string
  name: string
}

/** 平台信息（含端口权限提示等） */
export interface PlatformInfo {
  platform: string
  arch: string
  http01PrivilegeNote: string | null
  certsDirTemplate: string
}

/** 端口探测结果 */
export interface PortStatus {
  free: boolean
  code: 'free' | 'busy' | 'privilege' | 'error'
  detail: string | null
}

/** 设置项 */
export interface Settings {
  acme_directory: string // 'staging' | 'production'
  contact_email: string
  auto_renew: boolean
  run_at_login: boolean
  http01_port: number
  default_provider_id: number | null
  cert_key_type: string // 'rsa' | 'ecc'
  notify_expiring: boolean // 系统通知：证书到期提醒
  notify_renew_success: boolean // 系统通知：续期成功
  notify_renew_failed: boolean // 系统通知：续期失败
}

/** DNS Provider 配置（展示用，不含机密） */
export interface ProviderInfo {
  id: number
  kind: 'aliyun' | 'dnspod' | 'cloudflare'
  label: string
  enabled: boolean
  config: Record<string, string>
  created_at: string
}

/** Provider 新增/编辑入参 */
export interface ProviderInput {
  id?: number | null
  kind: 'aliyun' | 'dnspod' | 'cloudflare'
  label: string
  config: Record<string, string> // 含机密字段，如 access_key_secret
}

/** Provider 测试结果 */
export interface ProviderTestResult {
  ok: boolean
  message: string
  zone?: string
}

/** 证书信息 */
export interface CertInfo {
  id: number
  domain: string
  alt_names: string[]
  challenge_type: 'http01' | 'dns01'
  provider_id: number | null
  directory: 'staging' | 'production'
  status: 'issued' | 'renewing' | 'failed' | 'expired' | 'revoked'
  cert_chain_path: string
  private_key_path: string
  issuer: string | null
  issued_at: string
  expires_at: string
  days_remaining: number
  renew_after: string | null
  last_renewal_at: string | null
  last_error: string | null
  order_url: string | null
}

/** 申请请求入参 */
export interface IssueRequest {
  domain: string // 主域名，支持 *.example.com
  alt_names: string[] // 额外 SAN（不含主域名）
  challenge_type: 'http01' | 'dns01'
  provider_id: number | null // dns01 + api 模式时必填
  /** DNS-01 手动模式：用户自行添加解析记录，无需配置 DNS API */
  dns_manual?: boolean
  directory: 'staging' | 'production'
  contact_email: string
}

/** DNS 手动模式：单条 TXT 记录 */
export interface TxtRecord {
  domain: string
  recordName: string
  value: string
}

/** DNS 手动模式事件：需要用户手动添加的全部 TXT 记录 */
export interface TxtNeeded {
  jobId: string
  records: TxtRecord[]
}

/** 日志条目 */
export interface LogEntry {
  time: string
  level: string
  msg: string
}

/** 申请任务阶段 */
export type IssueStage =
  | 'InputValidated'
  | 'DirectoryReady'
  | 'AccountRegistered'
  | 'OrderCreated'
  | 'AuthorizationsFetched'
  | 'ChallengePrepared'
  | 'ChallengeServed'
  | 'ValidationInProgress'
  | 'Validated'
  | 'CsrSubmitted'
  | 'CertReady'
  | 'CertDownloaded'
  | 'CertInstalled'
  | 'RecordUpdated'
  | 'Completed'

/** 任务状态 */
export interface JobStatus {
  job_id: string
  state: 'pending' | 'running' | 'failed' | 'canceled' | 'completed'
  stage: IssueStage | null
  percent: number
  message: string | null
  error_code: string | null
  error_detail: string | null
  cert_id: number | null
  /** 任务目标域名（续期去重 / 悬浮条展示） */
  domain?: string | null
}

/** 进度事件 payload */
export interface JobProgress {
  job_id: string
  stage: IssueStage
  percent: number
  message: string
  detail?: string | null
}

/** 任务结束事件 payload（与 Rust finish_job 发出的一致） */
export interface JobFinished {
  job_id: string
  state: 'pending' | 'running' | 'failed' | 'canceled' | 'completed'
  ok: boolean
  cert_id: number | null
  error_code: string | null
  error_detail: string | null
}

/** 续期结果 */
export interface RenewalResult {
  cert_id: number
  domain: string
  ok: boolean
  message: string
}

/** 续期失败事件 payload */
export interface RenewalFailedPayload {
  domain: string
  message: string
  streak: number | null
}

/** 续期成功事件 payload */
export interface RenewalRenewedPayload {
  domain: string
  expires_at: string
}

/** 到期提醒事件 payload */
export interface RenewalExpiringPayload {
  level: '30' | '7' | '1' | 'expired'
  count: number
  domains: string[]
}

/** 手动检查续期完成事件 payload */
export interface RenewalCheckDonePayload {
  summary: string
  triggered: number
  failed: number
  skipped: number
}

/** 更新资产（安装包） */
export interface UpdateAsset {
  name: string
  url: string
  size: number
  sha256: string | null
}

/** 更新检查结果 */
export interface UpdateInfo {
  available: boolean
  currentVersion: string
  latestVersion: string
  tagName: string | null
  notes: string | null
  publishedAt: string | null
  asset: UpdateAsset | null
  releasePage: string
  /** 数据来源：domestic（国内清单）| github（兜底） */
  source: 'domestic' | 'github'
}

/** 更新下载进度事件 payload */
export interface UpdateProgress {
  received: number
  total: number
}

/** 错误码映射项 */
export interface ErrorInfo {
  code: string
  title: string
  message: string
  suggestion: string
  level: 'info' | 'warn' | 'error'
}
