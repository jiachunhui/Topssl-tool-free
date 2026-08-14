import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '../lib/api'
import type { Settings } from '../lib/types'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({
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
  })
  const loaded = ref(false)
  const error = ref<string | null>(null)

  async function fetchSettings() {
    try {
      settings.value = await api.getSettings()
      loaded.value = true
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  async function setKey(key: keyof Settings, value: string | number | boolean | null) {
    await api.setSetting(key, String(value))
    ;(settings.value as Record<string, unknown>)[key] = value
  }

  async function saveAll() {
    await api.setSettings(settings.value)
  }

  return { settings, loaded, error, fetchSettings, setKey, saveAll }
})
