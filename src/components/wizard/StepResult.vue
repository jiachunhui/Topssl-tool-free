<script setup lang="ts">
import { ref, watch } from 'vue'
import type { CertInfo } from '../../lib/types'
import { api } from '../../lib/api'
import { toast } from '../../lib/toast'

const props = defineProps<{
  cert: CertInfo | null
}>()
const emit = defineEmits<{ again: []; home: [] }>()

const guide = ref<string | null>(null)

watch(
  () => props.cert?.id,
  async (id) => {
    guide.value = null
    if (id == null) return
    try {
      guide.value = await api.getUsageGuide(id)
    } catch {
      guide.value = null
    }
  },
  { immediate: true },
)

async function copy(text: string, label: string) {
  try {
    await api.copyToClipboard(text)
    toast.success(`${label}已复制`)
  } catch {
    toast.error('复制失败')
  }
}
</script>

<template>
  <div v-if="cert" class="space-y-5 fade-in">
    <div class="flex flex-col items-center pt-4 text-center">
      <div class="flex h-14 w-14 items-center justify-center rounded-full bg-brand-100">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" class="h-7 w-7 text-brand-600">
          <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
        </svg>
      </div>
      <h2 class="mt-3 text-lg font-bold text-slate-900">证书申请成功！</h2>
      <p class="mt-1 font-mono text-sm text-slate-500">{{ cert.domain }} · 有效期至 {{ cert.expires_at.slice(0, 10) }}</p>
      <span v-if="cert.directory === 'staging'" class="mt-2 rounded bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700">
        注意：这是测试证书，不会被浏览器信任
      </span>
    </div>

    <div class="rounded-xl border border-slate-200 bg-white p-4">
      <div class="flex items-center justify-between text-sm">
        <span class="text-slate-400">证书目录</span>
        <div class="flex items-center gap-1.5">
          <code class="rounded bg-slate-100 px-2 py-1 text-xs text-slate-600">{{ cert.cert_chain_path.replace(/[\\/][^\\/]+$/, '') }}</code>
          <button class="btn-secondary !px-2.5 !py-1 text-xs" @click="api.openPath(cert.cert_chain_path.replace(/[\\/][^\\/]+$/, ''))">打开</button>
          <button
            class="btn-secondary !px-2.5 !py-1 text-xs"
            @click="copy(cert.cert_chain_path.replace(/[\\/][^\\/]+$/, ''), '路径')"
          >
            复制
          </button>
        </div>
      </div>
    </div>

    <div v-if="guide" class="rounded-xl border border-slate-200 bg-white p-4">
      <div class="mb-2 flex items-center justify-between">
        <span class="text-sm font-medium text-slate-700">在您的服务中引用</span>
        <button class="text-xs font-medium text-brand-600 hover:underline" @click="copy(guide, '指引')">复制全文</button>
      </div>
      <pre class="code-block whitespace-pre-wrap">{{ guide }}</pre>
    </div>

    <div class="flex justify-center gap-2 pt-2">
      <button class="btn-secondary" @click="emit('home')">返回证书列表</button>
      <button class="btn-brand" @click="emit('again')">继续申请下一个</button>
    </div>
  </div>
</template>
