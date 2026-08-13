import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '../lib/api'
import type { AppInfo, PlatformInfo } from '../lib/types'

export const useAppStore = defineStore('app', () => {
  const appInfo = ref<AppInfo | null>(null)
  const platformInfo = ref<PlatformInfo | null>(null)
  const globalLoading = ref(false)

  async function init() {
    const [app, platform] = await Promise.all([api.getAppInfo(), api.getPlatformInfo()])
    appInfo.value = app
    platformInfo.value = platform
  }

  return { appInfo, platformInfo, globalLoading, init }
})
