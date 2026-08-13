<script setup lang="ts">
import { onMounted } from 'vue'
import SideNav from './components/layout/SideNav.vue'
import ToastHost from './components/ui/ToastHost.vue'
import RunningJobBanner from './components/ui/RunningJobBanner.vue'
import { useAppStore } from './stores/app'
import { useCertsStore } from './stores/certs'
import { useSettingsStore } from './stores/settings'
import { onCertsChanged } from './lib/events'

const appStore = useAppStore()
const certsStore = useCertsStore()
const settingsStore = useSettingsStore()

onMounted(async () => {
  try {
    await appStore.init()
  } catch {
    // 非 Tauri 环境（纯浏览器预览）时忽略
  }
  certsStore.fetchCerts().catch(() => {})
  settingsStore.fetchSettings().catch(() => {})
  onCertsChanged(() => certsStore.fetchCerts())
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
