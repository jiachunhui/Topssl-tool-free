// 浏览器 Mock 层：非 Tauri 环境（纯浏览器）下模拟后端 command 与事件
// 用于开发阶段像网页一样测试 UI，数据用 localStorage 持久化
import type {
  CertInfo,
  IssueRequest,
  JobProgress,
  JobStatus,
  LogEntry,
  ProviderInfo,
  ProviderInput,
  Settings,
} from './types'

/** 是否运行在 Tauri（桌面应用）环境 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

// ---------- 简易事件总线（模拟 Tauri 事件） ----------
type Handler = (payload: unknown) => void
const eventHandlers = new Map<string, Set<Handler>>()

export const mockEvents = {
  on(event: string, cb: Handler): void {
    if (!eventHandlers.has(event)) eventHandlers.set(event, new Set())
    eventHandlers.get(event)!.add(cb)
  },
  off(event: string, cb: Handler): void {
    eventHandlers.get(event)?.delete(cb)
  },
  emit(event: string, payload: unknown): void {
    eventHandlers.get(event)?.forEach((cb) => {
      try {
        cb(payload)
      } catch {
        /* ignore */
      }
    })
  },
}

// ---------- 持久化存储 ----------
const PREFIX = 'mock:'

function lsGet(key: string): string | null {
  try {
    return localStorage.getItem(PREFIX + key)
  } catch {
    return null
  }
}
function lsSet(key: string, value: string): void {
  try {
    localStorage.setItem(PREFIX + key, value)
  } catch {
    /* ignore */
  }
}
function lsGetJSON<T>(key: string, fallback: T): T {
  const raw = lsGet(key)
  if (!raw) return fallback
  try {
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}
function lsSetJSON(key: string, value: unknown): void {
  lsSet(key, JSON.stringify(value))
}

const DEFAULT_SETTINGS: Settings = {
  acme_directory: 'staging',
  contact_email: '',
  auto_renew: true,
  run_at_login: true,
  http01_port: 80,
  default_provider_id: null,
  cert_key_type: 'rsa',
  notify_expiring: true,
  notify_renew_success: true,
  notify_renew_failed: true,
}

function loadSettings(): Settings {
  return { ...DEFAULT_SETTINGS, ...lsGetJSON<Partial<Settings>>('settings', {}) }
}
function saveSettings(s: Settings): void {
  lsSetJSON('settings', s)
}

function loadProviders(): ProviderInfo[] {
  return lsGetJSON<ProviderInfo[]>('providers', [])
}
function saveProviders(list: ProviderInfo[]): void {
  lsSetJSON('providers', list)
}

function loadCerts(): CertInfo[] {
  return lsGetJSON<CertInfo[]>('certs', [])
}
function saveCerts(list: CertInfo[]): void {
  lsSetJSON('certs', list)
}

function loadLogs(): LogEntry[] {
  return lsGetJSON<LogEntry[]>('logs', [])
}

let mockSeq = 1
function nextId(): number {
  return Date.now() % 1000000 + mockSeq++
}

// ---------- Mock 操作日志（写入 localStorage，问题3：日志页有内容） ----------
function log(level: string, msg: string): void {
  const entry: LogEntry = {
    time: new Date().toLocaleTimeString('zh-CN', { hour12: false }),
    level,
    msg,
  }
  const logs = loadLogs()
  logs.unshift(entry)
  if (logs.length > 500) logs.pop()
  lsSetJSON('logs', logs)
  mockEvents.emit('acme://logs-changed', entry)
}

function logError(e: unknown, ctx: string): void {
  console.error('[mock] ' + ctx + ' 出错:', e)
  log('ERROR', ctx + ' 出错: ' + (e instanceof Error ? e.message : String(e)))
}

// ---------- 模拟申请任务（驱动向导进度 UI） ----------
function simulateJob(req: IssueRequest, job_id: string): void {
  const steps: Array<{ stage: JobProgress['stage']; percent: number; message: string }> = [
    { stage: 'InputValidated', percent: 4, message: '域名校验通过' },
    { stage: 'DirectoryReady', percent: 12, message: '连接 ACME 目录…' },
    { stage: 'AccountRegistered', percent: 20, message: '注册 ACME 账户…' },
    { stage: 'OrderCreated', percent: 30, message: '创建证书订单…' },
    { stage: 'AuthorizationsFetched', percent: 42, message: '获取域名授权…' },
    { stage: 'ChallengePrepared', percent: 52, message: '准备验证…' },
  ]
  const isDnsManual = req.challenge_type === 'dns01' && !!req.dns_manual

  let i = 0
  const emitProgress = (): void => {
    if (i < steps.length) {
      const s = steps[i]
      i++
      const p: JobProgress = { job_id, stage: s.stage, percent: s.percent, message: s.message, detail: null }
      mockEvents.emit('acme://job-progress', p)
      setTimeout(emitProgress, 700)
      return
    }
    // 验证阶段
    if (isDnsManual) {
      const bare = req.domain.replace(/^\*\./, '')
      // 通配符 + 基础域名会生成两条同名不同值的 TXT（与真实 LE 行为一致），用于测试多卡片
      const records = [
        { domain: bare, recordName: '_acme-challenge.' + bare, value: 'mock_txt_value_AAA111aaa' },
        { domain: bare, recordName: '_acme-challenge.' + bare, value: 'mock_txt_value_BBB222bbb' },
      ]
      mockEvents.emit('acme://txt-needed', { jobId: job_id, records })
      // 等待 confirm_txt 后继续
      const waitConfirm = (): void => {
        setTimeout(() => {
          if (lsGetJSON<boolean>('txt-confirmed:' + job_id, false)) {
            continueValidate()
          } else {
            waitConfirm()
          }
        }, 800)
      }
      const continueValidate = (): void => {
        const p: JobProgress = { job_id, stage: 'ValidationInProgress', percent: 70, message: '等待 DNS 记录生效…', detail: null }
        mockEvents.emit('acme://job-progress', p)
        setTimeout(() => {
          const p2: JobProgress = { job_id, stage: 'Validated', percent: 76, message: req.domain + ' 验证通过（Mock）', detail: null }
          mockEvents.emit('acme://job-progress', p2)
          setTimeout(finish, 600)
        }, 900)
      }
      waitConfirm()
      return
    }
    const p: JobProgress = { job_id, stage: 'ValidationInProgress', percent: 70, message: '等待验证…（Mock）', detail: null }
    mockEvents.emit('acme://job-progress', p)
    setTimeout(() => {
      const p2: JobProgress = { job_id, stage: 'Validated', percent: 76, message: req.domain + ' 验证通过（Mock）', detail: null }
      mockEvents.emit('acme://job-progress', p2)
      setTimeout(finish, 600)
    }, 900)
  }
  const finish = (): void => {
    try {
      const p: JobProgress = { job_id, stage: 'Completed', percent: 92, message: '签发成功（Mock）', detail: null }
      mockEvents.emit('acme://job-progress', p)
      // 写入一张 mock 证书
      const certId = nextId()
      const certs = loadCerts()
      certs.unshift({
        id: certId,
        domain: req.domain,
        alt_names: req.alt_names ?? [],
        challenge_type: req.challenge_type,
        provider_id: req.provider_id,
        directory: req.directory,
        status: 'issued',
        cert_chain_path: 'mock/certs/' + req.domain + '/fullchain.pem',
        private_key_path: 'mock/certs/' + req.domain + '/privkey.pem',
        issuer: 'Mock Let\u2019s Encrypt',
        issued_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 90 * 86400 * 1000).toISOString(),
        days_remaining: 90,
        renew_after: null,
        last_renewal_at: null,
        last_error: null,
        order_url: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      } as CertInfo)
      saveCerts(certs)
      log('INFO', 'Mock 签发成功：' + req.domain)
      mockEvents.emit('certs://changed', {})
      mockEvents.emit('acme://job-finished', {
        job_id: job_id,
        ok: true,
        state: 'completed',
        cert_id: certId,
        error_code: null,
        error_detail: null,
      })
      console.log('[mock] finish 完成，已发出 job-finished:', job_id)
    } catch (e) {
      logError(e, '模拟签发')
      // 兜底：即使出错也发出完成事件，避免流程卡死
      mockEvents.emit('acme://job-finished', {
        job_id: job_id,
        ok: false,
        state: 'failed',
        cert_id: null,
        error_code: 'ERR_INTERNAL',
        error_detail: e instanceof Error ? e.message : String(e),
      })
    }
  }
  emitProgress()
}

// ---------- 主 mock 分发 ----------
export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const a = args ?? {}

  switch (cmd) {
    case 'get_app_info':
      return { version: '0.1.0', platform: 'browser', arch: 'x64', name: 'SSL 证书助手' } as T

    case 'get_platform_info':
      return {
        platform: 'browser',
        arch: 'x64',
        http01PrivilegeNote: '浏览器 Mock 模式：HTTP-01 验证为模拟行为，不会真实监听 80 端口',
        certsDirTemplate: '%APPDATA%/SSL-Cert-Desktop/certs',
      } as T

    case 'probe_port80':
      return { free: true, code: 'free', detail: 'Mock：80 端口可用（模拟）' } as T

    case 'open_path':
      console.log('[mock] open_path', a.path)
      return undefined as T

    case 'copy_to_clipboard':
      try {
        await navigator.clipboard.writeText(String(a.text ?? ''))
      } catch {
        /* ignore */
      }
      return undefined as T

    case 'get_settings':
      return loadSettings() as T

    case 'set_settings':
      saveSettings(a.settings as Settings)
      log('INFO', '保存设置（Mock）：' + JSON.stringify((a.settings as Settings).acme_directory))
      return undefined as T

    case 'set_setting': {
      const s = loadSettings()
      const key = a.key as string
      const value = String(a.value ?? '')
      if (key === 'acme_directory') s.acme_directory = value === 'production' ? 'production' : 'staging'
      else if (key === 'contact_email') s.contact_email = value
      else if (key === 'auto_renew') s.auto_renew = value === 'true'
      else if (key === 'run_at_login') s.run_at_login = value === 'true'
      else if (key === 'http01_port') s.http01_port = parseInt(value, 10) || 80
      else if (key === 'default_provider_id') s.default_provider_id = value ? parseInt(value, 10) : null
      else if (key === 'cert_key_type') s.cert_key_type = value === 'ecc' ? 'ecc' : 'rsa'
      else if (key === 'notify_expiring') s.notify_expiring = value === 'true'
      else if (key === 'notify_renew_success') s.notify_renew_success = value === 'true'
      else if (key === 'notify_renew_failed') s.notify_renew_failed = value === 'true'
      saveSettings(s)
      log('INFO', '设置项变更：' + key + '=' + value)
      return undefined as T
    }

    case 'get_logs':
      return loadLogs() as T

    case 'clear_logs':
      lsSet('logs', JSON.stringify([]))
      return undefined as T

    case 'list_providers':
      return loadProviders() as T

    case 'save_provider': {
      const cfg = a.cfg as ProviderInput
      const providers = loadProviders()
      if (cfg.id) {
        const idx = providers.findIndex((p) => p.id === cfg.id)
        if (idx >= 0) {
          const updated: ProviderInfo = {
            ...providers[idx],
            kind: cfg.kind,
            label: cfg.label,
            config: { ...providers[idx].config, ...cfg.config },
          }
          providers[idx] = updated
        }
      } else {
        providers.push({
          id: nextId(),
          kind: cfg.kind,
          label: cfg.label,
          enabled: true,
          config: cfg.config,
          created_at: new Date().toISOString(),
        })
      }
      saveProviders(providers)
      return (cfg.id ?? providers[providers.length - 1].id) as T
    }

    case 'test_provider':
      return { ok: true, message: 'Mock：配置校验通过（未真实调用服务商 API）', zone: 'example.com' } as T

    case 'delete_provider': {
      const providers = loadProviders().filter((p) => p.id !== a.id)
      saveProviders(providers)
      return undefined as T
    }

    case 'list_certificates':
    case 'list_certs':
    case 'get_certs':
      return loadCerts() as T

    case 'get_certificate': {
      const id = Number(a.id)
      return (loadCerts().find((c) => c.id === id) ?? null) as T
    }

    case 'delete_certificate': {
      saveCerts(loadCerts().filter((c) => c.id !== a.id))
      return undefined as T
    }

    case 'get_usage_guide':
      return 'Mock 使用指南：\n1. 部署 fullchain.pem\n2. 部署 private_key.pem\n3. 重启服务' as T

    case 'renew_now':
      return 'mock-renew-' + nextId() as T

    case 'check_renewals':
      return [] as T

    case 'check_duplicate':
      return { duplicate: false, certId: undefined } as T

    case 'start_issue': {
      const job_id = 'mock-job-' + nextId()
      lsSet('txt-confirmed:' + job_id, 'false')
      const req = a.req as IssueRequest
      const jobStatus: JobStatus = {
        job_id,
        state: 'running',
        stage: 'InputValidated',
        percent: 2,
        message: '任务已创建（Mock）',
        error_code: null,
        error_detail: null,
        cert_id: null,
        domain: req?.domain ?? null,
      }
      lsSetJSON('job-status:' + job_id, jobStatus)
      log('INFO', '开始申请（Mock）：' + (req.domain ?? '?') + ' [' + (req.challenge_type ?? '?') + ']')
      console.log('[mock] start_issue:', job_id, req)
      simulateJob(req, job_id)
      return job_id as T
    }

    case 'cancel_issue':
      return undefined as T

    case 'get_job_status': {
      const job_id = String(a.jobId ?? '')
      return lsGetJSON<JobStatus | null>('job-status:' + job_id, null) as T
    }

    case 'confirm_txt': {
      const job_id = String(a.jobId ?? '')
      lsSetJSON('txt-confirmed:' + job_id, true)
      return undefined as T
    }

    default:
      // 命令未实现时显式抛错，避免返回 undefined 导致 UI 静默崩溃（难以排查）
      console.warn('[mock] 未实现的 command:', cmd, a)
      throw new Error(`Mock 未实现命令: ${cmd}`)
  }
}
