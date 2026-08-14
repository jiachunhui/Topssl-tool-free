<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import SideNav from './components/layout/SideNav.vue'
import ToastHost from './components/ui/ToastHost.vue'
import RunningJobBanner from './components/ui/RunningJobBanner.vue'
import { useAppStore } from './stores/app'
import { useCertsStore } from './stores/certs'
import { useSettingsStore } from './stores/settings'
import {
  onCertsChanged,
  onRenewalCheckDone,
  onRenewalExpiring,
  onRenewalFailed,
  onRenewalRenewed,
} from './lib/events'
import { toast } from './lib/toast'

const appStore = useAppStore()
const certsStore = useCertsStore()
const settingsStore = useSettingsStore()
const router = useRouter()

onMounted(async () => {
  try {
    await appStore.init()
  } catch {
    // 非 Tauri 环境（纯浏览器预览）时忽略
  }
  certsStore.fetchCerts().catch(() => {})
  settingsStore.fetchSettings().catch(() => {})
  onCertsChanged(() => certsStore.fetchCerts())

  // 续期/到期通知的应用内提示（系统通知由后端发送；点击跳转到对应页面）
  onRenewalFailed((p) => {
    toast.error(`续期失败：${p.domain}（${p.message}）`, () => router.push('/logs'))
    certsStore.fetchCerts().catch(() => {})
  })
  onRenewalRenewed((p) => {
    toast.success(`证书续期成功：${p.domain}（新到期日 ${p.expires_at.slice(0, 10)}）`)
    certsStore.fetchCerts().catch(() => {})
  })
  onRenewalExpiring((p) => {
    const msg =
      p.level === 'expired'
        ? `${p.count} 张证书已过期，请立即处理`
        : `${p.count} 张证书将在 ${p.level} 天内到期`
    toast.warn(msg, () => router.push('/'))
    certsStore.fetchCerts().catch(() => {})
  })
  onRenewalCheckDone((p) => {
    toast.info(p.summary)
    certsStore.fetchCerts().catch(() => {})
  })
})
</script>

<template>
  <div class="flex h-screen overflow-hidden">
    <SideNav />
    <main class="flex-1 overflow-y-auto">
      <RouterView />
    </main>
    <RunningJobBanner />
    <ToastHost />
  </div>
</template>
