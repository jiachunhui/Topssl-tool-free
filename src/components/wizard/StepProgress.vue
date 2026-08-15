<script setup lang="ts">
import { computed, ref } from 'vue'
import ProgressSteps from '../ui/ProgressSteps.vue'
import type { JobProgress, JobStatus } from '../../lib/types'
import { getErrorInfo } from '../../lib/errors'
import { useJobStore } from '../../stores/job'
import { api } from '../../lib/api'
import { toast } from '../../lib/toast'

const props = defineProps<{
  progress: JobProgress | null
  status: JobStatus | null
}>()
const emit = defineEmits<{ cancel: []; retry: []; back: [] }>()

const jobStore = useJobStore()
const confirming = ref(false)

// 失败/取消时保留最后一次进度，避免跳回 0%；文案按状态给出合理默认（轻微问题 1）
const percent = computed(() => {
  if (props.progress) return props.progress.percent
  return props.status?.percent ?? 0
})
const message = computed(() => {
  if (props.progress?.message) return props.progress.message
  if (props.status?.message) return props.status.message
  if (props.status?.state === 'failed') return '申请失败'
  if (props.status?.state === 'canceled') return '任务已取消'
  return '正在准备…'
})

const errorInfo = computed(() => {
  const code = props.status?.error_code
  if (!code) return null
  return getErrorInfo(code, props.status?.error_detail)
})

const isCanceled = computed(() => props.status?.state === 'canceled')
const isFailed = computed(() => props.status?.state === 'failed')

async function copyTxt(text: string, label: string) {
  try {
    await api.copyToClipboard(text)
    toast.success(`${label}已复制`)
  } catch {
    toast.error('复制失败')
  }
}

async function confirmAdded() {
  confirming.value = true
  try {
    await jobStore.confirmTxtAdded()
    toast.success('好的，正在等待 DNS 生效并继续验证…')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '操作失败')
  } finally {
    confirming.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-bold text-slate-900">正在为您申请证书</h2>
      <p class="mt-1 text-sm text-slate-500">整个流程通常需要 1-3 分钟，请稍候</p>
    </div>

    <ProgressSteps :steps="['域名', '验证方式', '确认信息', '申请中']" :current="3" />

    <!-- DNS 手动模式：展示 TXT 记录等待用户添加（可能多条，须全部添加） -->
    <div v-if="jobStore.pendingTxts.length" class="rounded-xl border-2 border-amber-300 bg-amber-50 p-5 fade-in">
      <div class="flex items-center gap-2 text-sm font-bold text-amber-800">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" class="h-4.5 w-4.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" />
        </svg>
        请在 DNS 控制台添加以下 TXT 记录（共 {{ jobStore.pendingTxts.length }} 条）
      </div>
      <p class="mt-1.5 text-xs text-amber-700">
        前往您域名的 DNS 解析管理页面（如阿里云/腾讯云/Cloudflare 控制台），添加以下 <b>TXT</b> 类型记录（同名记录的多条都要全部添加），然后回来点击"我已添加"。
      </p>

      <div v-for="(t, idx) in jobStore.pendingTxts" :key="idx" class="mt-3 rounded-lg border border-amber-200 bg-white p-3">
        <div class="flex items-center justify-between text-xs text-slate-400">
          <span>记录 {{ idx + 1 }} · {{ t.domain }}</span>
          <span class="rounded bg-slate-100 px-2 py-0.5 font-mono text-xs font-bold text-slate-700">TXT</span>
        </div>
        <div class="mt-2 flex items-center justify-between gap-2">
          <span class="shrink-0 text-xs text-slate-400">主机记录 / 名称</span>
          <div class="flex min-w-0 items-center gap-1.5">
            <code class="code-block !p-1.5 !text-[11px]">{{ t.recordName }}</code>
            <button class="btn-secondary !px-2 !py-1 text-[11px]" @click="copyTxt(t.recordName, '记录名称')">复制</button>
          </div>
        </div>
        <div class="mt-2 flex items-center justify-between gap-2">
          <span class="shrink-0 text-xs text-slate-400">记录值</span>
          <div class="flex min-w-0 items-center gap-1.5">
            <code class="code-block !p-1.5 !text-[11px] break-all">{{ t.value }}</code>
            <button class="btn-secondary !px-2 !py-1 text-[11px]" @click="copyTxt(t.value, '记录值')">复制</button>
          </div>
        </div>
      </div>

      <div class="mt-4 flex items-center justify-end gap-2">
        <span class="text-[11px] text-amber-600">全部添加完成后点击继续，程序会自动检测记录生效</span>
        <button class="btn-brand" :disabled="confirming" @click="confirmAdded">
          {{ confirming ? '请稍候…' : '我已添加，继续验证' }}
        </button>
      </div>
    </div>

    <div class="rounded-xl border border-slate-200 bg-white p-5">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-slate-700">{{ message }}</span>
        <span class="text-sm font-bold text-brand-600">{{ percent }}%</span>
      </div>
      <div class="mt-2.5 h-2 overflow-hidden rounded-full bg-slate-100">
        <div class="h-full rounded-full bg-brand-500 transition-all duration-500" :style="{ width: percent + '%' }"></div>
      </div>

      <div v-if="isFailed && errorInfo" class="mt-4 rounded-lg border border-red-200 bg-red-50 p-4">
        <div class="flex items-center gap-2 text-sm font-semibold text-red-700">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-4.5 w-4.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
          </svg>
          {{ errorInfo.title }}
        </div>
        <p class="mt-1.5 text-sm text-red-600">{{ errorInfo.message }}</p>
        <p class="mt-1 text-xs text-red-500">建议：{{ errorInfo.suggestion }}</p>
      </div>

      <div v-if="isCanceled" class="mt-4 rounded-lg bg-slate-100 p-4 text-sm text-slate-600">申请任务已取消。</div>
    </div>

    <div class="flex justify-end gap-2">
      <button v-if="isFailed || isCanceled" class="btn-secondary" @click="emit('back')">返回修改</button>
      <button v-if="isFailed" class="btn-brand" @click="emit('retry')">重新申请</button>
      <button v-else-if="isCanceled" class="btn-brand" @click="emit('retry')">重新申请</button>
      <button v-else class="btn-secondary" @click="emit('cancel')">取消申请</button>
    </div>
  </div>
</template>
