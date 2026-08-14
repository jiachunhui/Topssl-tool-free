<script setup lang="ts">
import { ref, watch } from 'vue'
import type { CertInfo } from '../../lib/types'
import { api } from '../../lib/api'
import { toast } from '../../lib/toast'

const props = defineProps<{
  cert: CertInfo | null
}>()
const emit = defineEmits<{ close: [] }>()

const guide = ref<string | null>(null)
const loadingGuide = ref(false)
const certDir = ref('')

watch(
  () => props.cert?.id,
  async (id) => {
    guide.value = null
    certDir.value = ''
    if (id == null) return
    loadingGuide.value = true
    try {
      guide.value = await api.getUsageGuide(id)
      certDir.value = props.cert!.cert_chain_path.replace(/[\\/][^\\/]+$/, '')
    } catch (e) {
      guide.value = '（指引生成失败：' + (e instanceof Error ? e.message : String(e)) + '）'
    } finally {
      loadingGuide.value = false
    }
  },
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
  <Teleport to="body">
    <div v-if="cert" class="fixed inset-0 z-[80] flex items-center justify-center bg-slate-900/40 p-4" @click.self="emit('close')">
      <div class="flex max-h-[85vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl bg-white shadow-xl fade-in">
        <div class="flex items-center justify-between border-b border-slate-100 px-5 py-4">
          <div>
            <div class="flex items-center gap-2">
              <h3 class="font-mono text-sm font-bold text-slate-900">{{ cert.domain }}</h3>
              <span
                v-if="cert.directory === 'staging'"
                class="rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700"
                >测试证书</span
              >
            </div>
            <div class="mt-0.5 text-xs text-slate-400">申请于 {{ cert.issued_at.slice(0, 10) }} · 签发机构 {{ cert.issuer ?? 'Let\u2019s Encrypt' }}</div>
          </div>
          <button class="rounded-md p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600" @click="emit('close')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-5 w-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="flex-1 overflow-y-auto px-5 py-4">
          <div class="grid grid-cols-2 gap-3 text-sm">
            <div class="rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">到期时间</div>
              <div class="mt-0.5 font-medium text-slate-800">{{ cert.expires_at.slice(0, 10) }}（剩 {{ cert.days_remaining }} 天）</div>
            </div>
            <div class="rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">验证方式</div>
              <div class="mt-0.5 font-medium text-slate-800">{{ cert.challenge_type === 'http01' ? 'HTTP-01' : 'DNS-01' }}</div>
            </div>
            <div class="rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">最近续期</div>
              <div class="mt-0.5 font-medium text-slate-800">{{ cert.last_renewal_at ? cert.last_renewal_at.slice(0, 10) : '尚未续期' }}</div>
            </div>
            <div class="rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">下次续期</div>
              <div class="mt-0.5 font-medium text-slate-800">{{ cert.renew_after ? cert.renew_after.slice(0, 10) : '—' }}</div>
            </div>
            <div class="col-span-2 rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">证书目录</div>
              <div class="mt-1 flex items-center gap-2">
                <code class="code-block flex-1 !bg-slate-100 !p-2 !text-slate-700">{{ certDir }}</code>
                <button class="btn-secondary !px-2.5 !py-1.5 text-xs" @click="api.openPath(certDir)">打开</button>
                <button class="btn-secondary !px-2.5 !py-1.5 text-xs" @click="copy(certDir, '路径')">复制</button>
              </div>
            </div>
            <div class="col-span-2 rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">证书链文件</div>
              <div class="mt-1 font-mono text-xs break-all text-slate-700">{{ cert.cert_chain_path }}</div>
            </div>
            <div class="col-span-2 rounded-lg bg-slate-50 p-3">
              <div class="text-xs text-slate-400">私钥文件</div>
              <div class="mt-1 font-mono text-xs break-all text-slate-700">{{ cert.private_key_path }}</div>
            </div>
            <div v-if="cert.last_error" class="col-span-2 rounded-lg bg-red-50 p-3">
              <div class="text-xs text-red-500">最近错误</div>
              <div class="mt-1 text-xs text-red-700">{{ cert.last_error }}</div>
            </div>
          </div>

          <div class="mt-4">
            <h4 class="text-sm font-semibold text-slate-800">在您的服务中如何使用</h4>
            <pre v-if="guide" class="code-block mt-2 whitespace-pre-wrap">{{ guide }}</pre>
            <div v-else-if="loadingGuide" class="mt-2 animate-pulse rounded-lg bg-slate-100 p-4 text-xs text-slate-400">生成指引中…</div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
