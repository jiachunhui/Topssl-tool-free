<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { PortStatus } from '../../lib/types'
import { api } from '../../lib/api'
import { useProvidersStore } from '../../stores/providers'

const props = defineProps<{
  modelValue: 'http01' | 'dns01'
  providerId: number | null
  isWildcard: boolean
  dnsManual: boolean
}>()
const emit = defineEmits<{
  'update:modelValue': [v: 'http01' | 'dns01']
  'update:providerId': [v: number | null]
  'update:dnsManual': [v: boolean]
  next: []
  back: []
}>()

const router = useRouter()
const providersStore = useProvidersStore()

const challenge = ref<'http01' | 'dns01'>(props.isWildcard ? 'dns01' : props.modelValue)
const dnsManual = ref(props.dnsManual)
const portStatus = ref<PortStatus | null>(null)
const probing = ref(false)

// 与父级状态保持同步：返回上一步再进入时，单选展示与实际提交值一致（轻微问题 4）
watch(
  () => props.modelValue,
  (v) => {
    challenge.value = props.isWildcard ? 'dns01' : v
  },
)
watch(
  () => props.dnsManual,
  (v) => {
    dnsManual.value = v
  },
)

const providers = computed(() => providersStore.providers.filter((p) => p.enabled))

function pick(v: 'http01' | 'dns01') {
  if (props.isWildcard && v === 'http01') return
  challenge.value = v
  emit('update:modelValue', v)
}

function pickProvider(id: number) {
  emit('update:providerId', id)
}

function pickManual(v: boolean) {
  dnsManual.value = v
  emit('update:dnsManual', v)
  if (v) {
    emit('update:providerId', null)
  } else if (providers.value.length && props.providerId == null) {
    emit('update:providerId', providers.value[0].id)
  }
}

async function probe() {
  probing.value = true
  try {
    portStatus.value = await api.probePort80()
  } finally {
    probing.value = false
  }
}

onMounted(async () => {
  await providersStore.fetchProviders()
  if (!props.isWildcard) probe()
  // providerId 指向的服务商已被删除时清空，避免提交无效 id 导致后端报「服务商不存在」
  if (props.providerId != null && !providers.value.some((p) => p.id === props.providerId)) {
    emit('update:providerId', null)
  }
  if (providers.value.length && props.providerId == null) {
    emit('update:providerId', providers.value[0].id)
  }
})

function goProviderConfig() {
  router.push('/dns')
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-bold text-slate-900">选择验证方式</h2>
      <p class="mt-1 text-sm text-slate-500">验证域名归属后，Let's Encrypt 才会签发证书</p>
    </div>

    <div v-if="!isWildcard" class="grid gap-3 sm:grid-cols-2">
      <button
        type="button"
        class="rounded-xl border-2 p-4 text-left transition"
        :class="challenge === 'http01' ? 'border-brand-500 bg-brand-50/60' : 'border-slate-200 bg-white hover:border-slate-300'"
        @click="pick('http01')"
      >
        <div class="flex items-center justify-between">
          <span class="text-sm font-bold text-slate-900">HTTP 验证</span>
          <span v-if="challenge === 'http01'" class="rounded-full bg-brand-600 px-2 py-0.5 text-[10px] font-medium text-white">已选</span>
        </div>
        <p class="mt-1.5 text-xs leading-relaxed text-slate-500">
          自动在 80 端口临时开启验证服务，无需配置 API。
          <br />适用于云服务器等 80 端口公网可访问的环境；家庭宽带通常被封 80，请用 DNS 验证。
        </p>
        <div v-if="probing" class="mt-2 text-xs text-slate-400">检测 80 端口…</div>
        <div v-else-if="portStatus" class="mt-2 text-xs" :class="portStatus.free ? 'text-brand-600' : 'text-amber-600'">
          {{ portStatus.free ? '80 端口可用 ✓' : '80 端口不可用（' + (portStatus.detail ?? portStatus.code) + '）→ 建议 DNS 验证' }}
        </div>
      </button>

      <button
        type="button"
        class="rounded-xl border-2 p-4 text-left transition"
        :class="challenge === 'dns01' ? 'border-brand-500 bg-brand-50/60' : 'border-slate-200 bg-white hover:border-slate-300'"
        @click="pick('dns01')"
      >
        <div class="flex items-center justify-between">
          <span class="text-sm font-bold text-slate-900">DNS 验证（推荐）</span>
          <span v-if="challenge === 'dns01'" class="rounded-full bg-brand-600 px-2 py-0.5 text-[10px] font-medium text-white">已选</span>
        </div>
        <p class="mt-1.5 text-xs leading-relaxed text-slate-500">
          通过添加 TXT 解析记录验证，无需 80 端口，支持通配符，国内通用。
          <br />可自动调用 DNS API，或手动添加记录。
        </p>
      </button>
    </div>

    <div v-else class="rounded-xl border-2 border-brand-500 bg-brand-50/60 p-4">
      <div class="text-sm font-bold text-slate-900">通配符证书 → 只能使用 DNS 验证</div>
      <p class="mt-1 text-xs text-slate-500">通配符域名（*.example.com）无法通过 HTTP 验证，请选择 DNS 验证方式。</p>
    </div>

    <div v-if="challenge === 'dns01'" class="space-y-3">
      <div class="rounded-xl border border-slate-200 bg-white p-4">
        <div class="flex items-center gap-2">
          <input id="dns-auto" type="radio" class="accent-brand-600" :checked="!dnsManual" @change="pickManual(false)" />
          <label for="dns-auto" class="text-sm font-medium text-slate-800">自动添加记录（推荐）</label>
          <span class="ml-auto rounded bg-slate-100 px-1.5 py-0.5 text-[11px] text-slate-500">需要 DNS 服务商 API</span>
        </div>
        <p class="mt-1 pl-6 text-xs text-slate-500">配置一次服务商 API（阿里云/DNSPod/Cloudflare），程序自动添加与清理验证记录。</p>

        <div v-if="!dnsManual" class="mt-3 pl-6">
          <label class="mb-1.5 block text-sm font-medium text-slate-700">选择 DNS 服务商</label>
          <div v-if="providers.length" class="space-y-2">
            <label
              v-for="p in providers"
              :key="p.id"
              class="flex cursor-pointer items-center gap-3 rounded-lg border border-slate-200 bg-white px-3.5 py-2.5 text-sm transition hover:border-brand-400"
              :class="props.providerId === p.id ? 'border-brand-500 bg-brand-50/50' : ''"
            >
              <input type="radio" class="accent-brand-600" :checked="props.providerId === p.id" @change="pickProvider(p.id)" />
              <span class="font-medium text-slate-800">{{ p.label }}</span>
              <span class="ml-auto rounded bg-slate-100 px-1.5 py-0.5 text-[11px] text-slate-500">
                {{ p.kind === 'aliyun' ? '阿里云' : p.kind === 'dnspod' ? 'DNSPod/腾讯云' : 'Cloudflare' }}
              </span>
            </label>
          </div>
          <div v-else class="rounded-lg border border-dashed border-slate-300 bg-slate-50 p-4 text-center">
            <p class="text-sm text-slate-500">还没有配置 DNS 服务商</p>
            <button class="btn-brand mt-3" @click="goProviderConfig">去配置</button>
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-slate-200 bg-white p-4">
        <div class="flex items-center gap-2">
          <input id="dns-manual" type="radio" class="accent-brand-600" :checked="dnsManual" @change="pickManual(true)" />
          <label for="dns-manual" class="text-sm font-medium text-slate-800">手动添加解析记录</label>
          <span class="ml-auto rounded bg-slate-100 px-1.5 py-0.5 text-[11px] text-slate-500">无需 API</span>
        </div>
        <p class="mt-1 pl-6 text-xs text-slate-500">
          申请过程中程序会给出需要添加的 TXT 记录（名称和值），您到域名控制台手动添加后，回到程序点击"我已添加"继续。适用于没有 API 权限的情况。
        </p>
      </div>
    </div>

    <div class="flex justify-between pt-2">
      <button class="btn-secondary" @click="emit('back')">上一步</button>
      <button
        class="btn-brand"
        :disabled="challenge === 'dns01' && !dnsManual && !props.providerId"
        @click="emit('next')"
      >
        下一步
      </button>
    </div>
  </div>
</template>
