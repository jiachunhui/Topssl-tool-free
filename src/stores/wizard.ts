import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CertInfo } from '../lib/types'

// 申请向导的表单与界面状态（全局化：切换栏目离开 /wizard 后不丢失，
// 返回时配合 job store 自动恢复进行中的任务界面）
export const useWizardStore = defineStore('wizard', () => {
  const step = ref(0) // 0 域名 / 1 验证 / 2 确认 / 3 进度 / 4 结果
  const domain = ref('')
  const altNames = ref<string[]>([])
  const challengeType = ref<'http01' | 'dns01'>('dns01')
  const providerId = ref<number | null>(null)
  const dnsManual = ref(false)
  const directory = ref<'staging' | 'production'>('staging')
  const email = ref('')
  const resultCert = ref<CertInfo | null>(null)
  const submitting = ref(false)

  /** 重置向导（重新申请 / 申请下一个域名）；directory/email 来自全局设置，保留不清 */
  function resetAll() {
    step.value = 0
    domain.value = ''
    altNames.value = []
    challengeType.value = 'dns01'
    providerId.value = null
    dnsManual.value = false
    resultCert.value = null
    submitting.value = false
  }

  return {
    step,
    domain,
    altNames,
    challengeType,
    providerId,
    dnsManual,
    directory,
    email,
    resultCert,
    submitting,
    resetAll,
  }
})
