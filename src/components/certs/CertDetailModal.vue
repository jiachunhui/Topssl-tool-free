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

// 部署相关状态
const deployBusy = ref(false)
const showIisPanel = ref(false)
const iisLoading = ref(false)
const iisSites = ref<{ name: string }[]>([])
const iisSite = ref('')
const iisHost = ref('')

watch(
  () => props.cert?.id,
  async (id) => {
    guide.value = null
    certDir.value = ''
    showIisPanel.value = false
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

/** 导出部署包到下载目录并打开 */
async function exportPkg() {
  if (!props.cert) return
  deployBusy.value = true
  try {
    const dir = await api.exportDeployPackage(props.cert.id)
    toast.success('部署包已生成，正在打开文件夹')
    await api.openPath(dir)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '导出失败')
  } finally {
    deployBusy.value = false
  }
}

/** 打开 IIS 部署面板：先检测 IIS 与权限 */
async function openIisPanel() {
  if (!props.cert) return
  showIisPanel.value = true
  iisLoading.value = true
  try {
    const st = await api.iisStatus()
    if (!st.supported) {
      toast.error('IIS 部署仅支持 Windows 系统')
      showIisPanel.value = false
      return
    }
    if (!st.installed) {
      toast.error('未检测到 IIS，请先在「Windows 功能」中安装 IIS')
      showIisPanel.value = false
      return
    }
    if (!st.elevated) {
      toast.error('需要管理员权限：请右键以管理员身份运行 Tossl 后重试')
      showIisPanel.value = false
      return
    }
    iisSites.value = st.sites
    iisSite.value = st.sites[0]?.name ?? ''
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'IIS 检测失败')
    showIisPanel.value = false
  } finally {
    iisLoading.value = false
  }
}

/** 执行 IIS 一键部署 */
async function doIisDeploy() {
  if (!props.cert) return
  deployBusy.value = true
  try {
    const msg = await api.iisDeployCert(props.cert.id, iisSite.value, iisHost.value)
    toast.success(msg)
    showIisPanel.value = false
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '部署失败')
  } finally {
    deployBusy.value = false
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

            <div class="col-span-2 flex flex-wrap gap-2">
              <button class="btn-brand !px-3 !py-1.5 text-xs" :disabled="deployBusy" @click="exportPkg">
                {{ deployBusy ? '生成中…' : '导出部署包' }}
              </button>
              <button class="btn-secondary !px-3 !py-1.5 text-xs" :disabled="deployBusy" @click="openIisPanel">
                IIS 一键部署
              </button>
            </div>

            <div v-if="showIisPanel" class="col-span-2 rounded-lg border border-slate-200 bg-slate-50 p-3">
              <div class="text-xs font-semibold text-slate-700">选择 IIS 站点</div>
              <div v-if="iisLoading" class="mt-2 text-xs text-slate-400">检测 IIS 中…</div>
              <div v-else-if="iisSites.length" class="mt-2 space-y-1.5">
                <label v-for="s in iisSites" :key="s.name" class="flex cursor-pointer items-center gap-2 text-sm">
                  <input type="radio" :value="s.name" v-model="iisSite" class="accent-brand-600" />
                  <span class="text-slate-700">{{ s.name }}</span>
                </label>
                <input
                  v-model="iisHost"
                  type="text"
                  placeholder="主机名（留空使用证书域名）"
                  class="mt-2 w-full rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs outline-none transition focus:border-brand-500"
                />
                <button
                  class="btn-brand mt-2 !px-3 !py-1.5 text-xs"
                  :disabled="!iisSite || deployBusy"
                  @click="doIisDeploy"
                >
                  {{ deployBusy ? '部署中…' : '开始部署' }}
                </button>
                <p class="mt-2 text-[11px] leading-relaxed text-slate-400">
                  将证书导入本机证书库，并为所选站点添加 https 绑定（443 端口）与证书关联。
                </p>
              </div>
              <div v-else class="mt-2 text-xs text-amber-700">未检测到 IIS 站点</div>
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
