// 类型化 Tauri IPC 封装：统一错误归一化
// 浏览器（非 Tauri）环境自动切换到 Mock 实现，便于像网页一样测试 UI
import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { getErrorInfo } from './errors'
import type { ErrorInfo } from './types'
import { isTauri, mockInvoke } from './mock'

/** 后端抛出的错误结构（Rust AppError 序列化） */
export interface BackendError {
  code: string
  message: string
  detail?: string | null
}

/** 归一化后的错误：附中文文案（含操作建议） */
export class AppError extends Error {
  code: string
  info: ErrorInfo
  raw: unknown

  constructor(code: string, raw: unknown, detail?: string | null) {
    const info = getErrorInfo(code, detail)
    super(info.suggestion ? `${info.title}: ${info.message}（${info.suggestion}）` : `${info.title}: ${info.message}`)
    this.name = 'AppError'
    this.code = code
    this.info = info
    this.raw = raw
  }
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    // 浏览器 Mock 模式
    return mockInvoke<T>(cmd, args)
  }
  try {
    return await tauriInvoke<T>(cmd, args)
  } catch (e) {
    throw normalizeError(e)
  }
}

function normalizeError(e: unknown): AppError {
  // Tauri 2 的错误可能是字符串（"code"）或 {code, message} 或 {message: "code"}
  if (typeof e === 'string') {
    return new AppError(e, e)
  }
  if (e && typeof e === 'object') {
    const obj = e as Record<string, unknown>
    const code = (obj.code ?? obj.message) as string | undefined
    if (code && typeof code === 'string' && code.startsWith('ERR_')) {
      const detail = typeof obj.detail === 'string' ? obj.detail : null
      return new AppError(code, e, detail)
    }
    if (obj.message && typeof obj.message === 'string') {
      return new AppError(obj.message, e)
    }
  }
  return new AppError('ERR_INTERNAL', e)
}
