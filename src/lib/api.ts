// 所有 Tauri command 的类型化封装
import { invoke } from './ipc'
import type {
  AppInfo,
  CertInfo,
  IssueRequest,
  LogEntry,
  PlatformInfo,
  PortStatus,
  ProviderInfo,
  ProviderInput,
  ProviderTestResult,
  RenewalResult,
  Settings,
  JobStatus,
} from './types'

// ---------- system ----------
export const api = {
  getAppInfo: () => invoke<AppInfo>('get_app_info'),
  getPlatformInfo: () => invoke<PlatformInfo>('get_platform_info'),
  probePort80: () => invoke<PortStatus>('probe_port80'),
  openPath: (path: string) => invoke<void>('open_path', { path }),
  copyToClipboard: (text: string) => invoke<void>('copy_to_clipboard', { text }),
  getLogs: (limit?: number) => invoke<LogEntry[]>('get_logs', { limit: limit ?? 300 }),
  clearLogs: () => invoke<void>('clear_logs'),
  /** 前端把运行错误写入应用日志（排查用） */
  frontendLog: (level: 'info' | 'warn' | 'error', msg: string) =>
    invoke<void>('frontend_log', { level, msg }),

  // ---------- settings ----------
  getSettings: () => invoke<Settings>('get_settings'),
  setSetting: (key: string, value: string) => invoke<void>('set_setting', { key, value }),
  setSettings: (settings: Settings) => invoke<void>('set_settings', { settings }),

  // ---------- certificates ----------
  listCertificates: () => invoke<CertInfo[]>('list_certificates'),
  getCertificate: (id: number) => invoke<CertInfo | null>('get_certificate', { id }),
  deleteCertificate: (id: number) => invoke<void>('delete_certificate', { id }),
  getUsageGuide: (id: number) => invoke<string>('get_usage_guide', { id }),
  renewNow: (id: number) => invoke<string>('renew_now', { id }),
  checkRenewals: (force?: boolean) => invoke<RenewalResult[]>('check_renewals', { force: force ?? false }),

  // ---------- providers ----------
  listProviders: () => invoke<ProviderInfo[]>('list_providers'),
  saveProvider: (cfg: ProviderInput) => invoke<number>('save_provider', { cfg }),
  testProvider: (id: number) => invoke<ProviderTestResult>('test_provider', { id }),
  deleteProvider: (id: number) => invoke<void>('delete_provider', { id }),

  // ---------- issue ----------
  startIssue: (req: IssueRequest) => invoke<string>('start_issue', { req }),
  cancelIssue: (jobId: string) => invoke<void>('cancel_issue', { jobId }),
  getJobStatus: (jobId: string) => invoke<JobStatus | null>('get_job_status', { jobId }),
  confirmTxt: (jobId: string) => invoke<void>('confirm_txt', { jobId }),
  checkDuplicate: (domain: string, directory?: string) =>
    invoke<{ duplicate: boolean; certId?: number }>('check_duplicate', {
      domain,
      directory: directory ?? null,
    }),
}
