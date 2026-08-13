import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '../lib/api'
import type { CertInfo } from '../lib/types'

export const useCertsStore = defineStore('certs', () => {
  const certs = ref<CertInfo[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchCerts() {
    loading.value = true
    error.value = null
    try {
      certs.value = await api.listCertificates()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function remove(id: number) {
    await api.deleteCertificate(id)
    certs.value = certs.value.filter((c) => c.id !== id)
  }

  async function renew(id: number) {
    return api.renewNow(id)
  }

  return { certs, loading, error, fetchCerts, remove, renew }
})
