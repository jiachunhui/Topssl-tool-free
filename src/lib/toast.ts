// 轻量全局 Toast（响应式 + 自动消失）
import { reactive } from 'vue'

export interface ToastItem {
  id: number
  type: 'success' | 'error' | 'info'
  message: string
}

export const toastState = reactive<{ items: ToastItem[] }>({ items: [] })

let seq = 0

function push(type: ToastItem['type'], message: string, duration = 3200) {
  const id = ++seq
  toastState.items.push({ id, type, message })
  setTimeout(() => {
    const i = toastState.items.findIndex((t) => t.id === id)
    if (i >= 0) toastState.items.splice(i, 1)
  }, duration)
}

export const toast = {
  success: (m: string) => push('success', m),
  error: (m: string) => push('error', m, 5000),
  info: (m: string) => push('info', m),
}
