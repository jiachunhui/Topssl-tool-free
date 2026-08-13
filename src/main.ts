import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import App from './App.vue'
import router from './router'
import { api } from './lib/api'

const app = createApp(App)
app.use(createPinia())
app.use(router)

// 前端运行错误写入应用日志（可在"应用日志"页查看）
app.config.errorHandler = (err, _instance, info) => {
  console.error('[vue]', info, err)
  const msg = `[vue] ${info}: ${err instanceof Error ? err.message : String(err)}`
  api.frontendLog('error', msg).catch(() => {})
}
if (typeof window !== 'undefined') {
  window.addEventListener('unhandledrejection', (e) => {
    const msg = `[promise] ${e.reason instanceof Error ? e.reason.message : String(e.reason)}`
    api.frontendLog('error', msg).catch(() => {})
  })
}

app.mount('#app')
