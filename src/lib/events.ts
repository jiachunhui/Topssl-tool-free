// Tauri 事件订阅封装（自动清理）
// 浏览器（非 Tauri）环境使用 Mock 事件总线
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { JobFinished, JobProgress, TxtNeeded } from './types'
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
