<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import ProgressSteps from '../components/ui/ProgressSteps.vue'
import StepDomain from '../components/wizard/StepDomain.vue'
import StepChallenge from '../components/wizard/StepChallenge.vue'
import StepEnvironment from '../components/wizard/StepEnvironment.vue'
import StepProgress from '../components/wizard/StepProgress.vue'
import StepResult from '../components/wizard/StepResult.vue'
import { useJobStore } from '../stores/job'
import { useSettingsStore } from '../stores/settings'
import { useCertsStore } from '../stores/certs'
import { useWizardStore } from '../stores/wizard'
import { toast } from '../lib/toast'
import type { IssueRequest } from '../lib/types'

const router = useRouter()
const jobStore = useJobStore()
const settingsStore = useSettingsStore()
const certsStore = useCertsStore()
const wizard = useWizardStore()

const {
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
} = storeToRefs(wizard)

const isWildcard = () => domain.value.startsWith('*.')

onMounted(async () => {
  await settingsStore.fetchSettings()
  // 仅在初始（未开始填写）时用设置里的默认值
  if (step.value === 0) {
    directory.value = settingsStore.settings.acme_directory === 'production' ? 'production' : 'staging'
    email.value = settingsStore.settings.contact_email ?? ''
    // 初始时默认选中设置里配置的默认 DNS 服务商（让 default_provider_id 设置真正生效，M7）
    if (providerId.value == null && settingsStore.settings.default_provider_id) {
      providerId.value = settingsStore.settings.default_provider_id
    }
  }
  // 返回向导时根据全局任务状态恢复对应界面
  resumeJobView()
})

/** 返回向导时恢复任务界面（仅进行中 / 已完成需要恢复；失败状态不劫持页面，由用户自行处理） */
async function resumeJobView() {
  const st = jobStore.status
  if (jobStore.running || st?.state === 'running' || st?.state === 'pending') {
    // 任务进行中：直接进入进度页（进度/TXT 卡片来自全局 job store）
    step.value = 3
    return
  }
  if (st?.state === 'completed') {
    // 离开期间任务已完成：拉取证书并显示结果页
    await certsStore.fetchCerts()
    const c =
      certsStore.certs.find((x) => x.id === jobStore.finishedCertId) ??
      certsStore.certs.find((x) => x.domain === domain.value) ??
      null
    resultCert.value = c
    submitting.value = false
    step.value = 4
  }
}

async function startIssue() {
  // 双击/重复提交防护
  if (submitting.value) return
  submitting.value = true
  step.value = 3
  try {
    const req: IssueRequest = {
      domain: domain.value,
      alt_names: altNames.value,
      challenge_type: challengeType.value,
      provider_id: challengeType.value === 'dns01' && !dnsManual.value ? providerId.value : null,
      dns_manual: challengeType.value === 'dns01' ? dnsManual.value : false,
      directory: directory.value,
      contact_email: email.value.trim(),
    }
    await jobStore.start(req)
  } catch (e) {
    submitting.value = false
    step.value = 2
    toast.error(e instanceof Error ? e.message : String(e))
  }
}

function retry() {
  resultCert.value = null
  jobStore.reset()
  startIssue()
}

/** 返回修改：清除旧任务状态，避免残留失败状态干扰后续操作 */
function backToEdit() {
  jobStore.reset()
  // 兜底复位提交中标记，避免失败事件丢失时「开始申请」按钮被永久禁用
  submitting.value = false
  step.value = 2
}

function again() {
  jobStore.reset()
  wizard.resetAll()
}

function onJobFinished() {
  // 任务结束后拉取最新证书列表，找到本次申请的证书（优先用事件携带的 cert_id）
  setTimeout(async () => {
    await certsStore.fetchCerts()
    const c =
      certsStore.certs.find((x) => x.id === jobStore.finishedCertId) ??
      certsStore.certs.find((x) => x.domain === domain.value) ??
      null
    resultCert.value = c
    step.value = 4
    submitting.value = false
  }, 300)
}

// 订阅 job 状态：done 时跳结果页（响应式 watch 替代轮询，更可靠）
watch(
  () => jobStore.status,
  (st) => {
    if (step.value !== 3) return
    if (st?.state === 'completed') {
      onJobFinished()
    } else if (st?.state === 'failed' || st?.state === 'canceled') {
      submitting.value = false
      // 停留在进度页展示错误
    }
  },
)
</script>

<template>
  <div class="mx-auto max-w-2xl px-6 py-8">
    <div v-if="step < 3" class="mb-6">
      <ProgressSteps :steps="['域名', '验证方式', '确认信息', '申请中']" :current="step" />
    </div>

    <div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <StepDomain
        v-if="step === 0"
        :domain="domain"
        :alt-names="altNames"
        :directory="directory"
        @update="(d, a) => ((domain = d), (altNames = a))"
        @next="step = 1"
      />
      <StepChallenge
        v-else-if="step === 1"
        :model-value="challengeType"
        :provider-id="providerId"
        :is-wildcard="isWildcard()"
        :dns-manual="dnsManual"
        @update:model-value="(v) => (challengeType = v)"
        @update:provider-id="(v) => (providerId = v)"
        @update:dns-manual="(v) => (dnsManual = v)"
        @back="step = 0"
        @next="step = 2"
      />
      <StepEnvironment
        v-else-if="step === 2"
        :domain="domain"
        :alt-names="altNames"
        :challenge-type="challengeType"
        :directory="directory"
        :email="email"
        :submitting="submitting"
        @update:directory="(v) => (directory = v)"
        @update:email="(v) => (email = v)"
        @back="step = 1"
        @next="startIssue"
      />
      <StepProgress
        v-else-if="step === 3"
        :progress="jobStore.progress"
        :status="jobStore.status"
        @cancel="jobStore.cancel()"
        @retry="retry"
        @back="backToEdit"
      />
      <StepResult v-else-if="step === 4 && resultCert" :cert="resultCert" @home="router.push('/')" @again="again" />
    </div>
  </div>
</template>
