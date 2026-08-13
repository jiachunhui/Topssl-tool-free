import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '../lib/api'
import type { ProviderInfo, ProviderInput, ProviderTestResult } from '../lib/types'

export const useProvidersStore = defineStore('providers', () => {
  const providers = ref<ProviderInfo[]>([])
  const loading = ref(false)

  async function fetchProviders() {
    loading.value = true
    try {
      providers.value = await api.listProviders()
    } finally {
      loading.value = false
    }
  }

  async function save(input: ProviderInput): Promise<number> {
    const id = await api.saveProvider(input)
    await fetchProviders()
    return id
  }

  async function remove(id: number) {
    await api.deleteProvider(id)
    providers.value = providers.value.filter((p) => p.id !== id)
  }

  async function test(id: number): Promise<ProviderTestResult> {
    return api.testProvider(id)
  }

  return { providers, loading, fetchProviders, save, remove, test }
})
