// 轻量全局 Toast（响应式 + 自动消失 + 可选点击动作）
import { reactive } from 'vue'

export interface ToastItem {
  id: number
  type: 'success' | 'error' | 'info' | 'warn'
  message: string
  /** 点击 toast 时的动作（如跳转到日志页） */
  onClick?: () => void
}

export const toastState = reactive<{ items: ToastItem[] }>({ items: [] })

let seq = 0

function push(type: ToastItem['type'], message: string, duration = 3200, onClick?: () => void) {
  const id = ++seq
  toastState.items.push({ id, type, message, onClick })
  setTimeout(() => {
    const i = toastState.items.findIndex((t) => t.id === id)
    if (i >= 0) toastState.items.splice(i, 1)
  }, duration)
}

export const toast = {
  success: (m: string, onClick?: () => void) => push('success', m, 3200, onClick),
  error: (m: string, onClick?: () => void) => push('error', m, 5000, onClick),
  warn: (m: string, onClick?: () => void) => push('warn', m, 5000, onClick),
  info: (m: string, onClick?: () => void) => push('info', m, 3200, onClick),
}
