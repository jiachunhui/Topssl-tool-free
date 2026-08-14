// Tauri 事件订阅封装（自动清理）
// 浏览器（非 Tauri）环境使用 Mock 事件总线
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  JobFinished,
  JobProgress,
  RenewalCheckDonePayload,
  RenewalExpiringPayload,
  RenewalFailedPayload,
  RenewalRenewedPayload,
  TxtNeeded,
} from './types'
import { isTauri, mockEvents } from './mock'

export interface EventSubscription {
  unlisten: UnlistenFn
}

async function subscribe<T>(event: string, cb: (payload: T) => void): Promise<EventSubscription> {
  if (!isTauri()) {
    const handler = (payload: unknown): void => cb(payload as T)
    mockEvents.on(event, handler)
    return { unlisten: () => mockEvents.off(event, handler) as unknown as UnlistenFn }
  }
  const unlisten = await listen<T>(event, (e) => cb(e.payload))
  return { unlisten }
}

/** 订阅申请任务进度 */
export function onJobProgress(cb: (p: JobProgress) => void): Promise<EventSubscription> {
  return subscribe<JobProgress>('acme://job-progress', cb)
}

/** 订阅任务结束 */
export function onJobFinished(cb: (p: JobFinished) => void): Promise<EventSubscription> {
  return subscribe<JobFinished>('acme://job-finished', cb)
}

/** 订阅证书列表变更 */
export function onCertsChanged(cb: () => void): Promise<EventSubscription> {
  return subscribe('certs://changed', cb)
}

/** 订阅 DNS 手动模式：需要用户添加 TXT 记录 */
export function onTxtNeeded(cb: (p: TxtNeeded) => void): Promise<EventSubscription> {
  return subscribe<TxtNeeded>('acme://txt-needed', cb)
}

/** 订阅续期失败（后端系统通知同步发出的应用内提示） */
export function onRenewalFailed(cb: (p: RenewalFailedPayload) => void): Promise<EventSubscription> {
  return subscribe<RenewalFailedPayload>('renewal://failed', cb)
}

/** 订阅续期成功 */
export function onRenewalRenewed(cb: (p: RenewalRenewedPayload) => void): Promise<EventSubscription> {
  return subscribe<RenewalRenewedPayload>('renewal://renewed', cb)
}

/** 订阅到期提醒（分级：30 / 7 / 1 天 + 已过期） */
export function onRenewalExpiring(cb: (p: RenewalExpiringPayload) => void): Promise<EventSubscription> {
  return subscribe<RenewalExpiringPayload>('renewal://expiring', cb)
}

/** 订阅手动"立即检查续期"完成 */
export function onRenewalCheckDone(cb: (p: RenewalCheckDonePayload) => void): Promise<EventSubscription> {
  return subscribe<RenewalCheckDonePayload>('renewal://check-done', cb)
}
