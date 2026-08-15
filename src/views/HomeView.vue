<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import CertCard from '../components/certs/CertCard.vue'
import CertDetailModal from '../components/certs/CertDetailModal.vue'
import ConfirmDialog from '../components/ui/ConfirmDialog.vue'
import { useCertsStore } from '../stores/certs'
import { useJobStore } from '../stores/job'
import { toast } from '../lib/toast'
import { topsslUrl, openExternal, ENTERPRISE_PRICE } from '../lib/promo'
import type { CertInfo } from '../lib/types'

const router = useRouter()
const certsStore = useCertsStore()
const jobStore = useJobStore()

const detailCert = ref<CertInfo | null>(null)
const confirmRef = ref<InstanceType<typeof ConfirmDialog> | null>(null)
const pendingRemove = ref<CertInfo | null>(null)

const filter = ref<'all' | 'valid' | 'expiring' | 'expired'>('all')

const filteredCerts = () => {
  const list = certsStore.certs
  switch (filter.value) {
    case 'valid':
      return list.filter((c) => c.status === 'issued' && c.days_remaining > 30)
    case 'expiring':
      return list.filter((c) => c.status === 'issued' && c.days_remaining <= 30)
    case 'expired':
      return list.filter((c) => c.status === 'expired' || c.status === 'failed')
    default:
      return list
  }
}

async function onRenew(cert: CertInfo) {
  try {
    const jobId = await certsStore.renew(cert.id)
    jobStore.restore(jobId, cert.domain)
    toast.info(`已开始为 ${cert.domain} 续期`)
    // 跳转到向导页查看续期进度；手动 DNS 证书的续期需要在这里添加并确认 TXT 记录（B1）
    router.push('/wizard')
    setTimeout(() => certsStore.fetchCerts(), 1000)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '续期失败')
  }
}

async function onRemove(cert: CertInfo) {
  pendingRemove.value = cert
  const ok = await confirmRef.value?.ask()
  if (!ok) return
  try {
    await certsStore.remove(cert.id)
    toast.success(`已删除 ${cert.domain} 的记录`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '删除失败')
  }
}

/** 企业证书引导卡片（TopSSL 曝光点：我的证书页，1-3 张） */
const promoCards = [
  {
    title: '单域名证书',
    desc: `价格更实惠，低至 ¥${ENTERPRISE_PRICE}/年，一次购买多年免去频繁续期`,
    url: topsslUrl('app-home', 'single-domain', '/ssl/one'),
  },
  {
    title: '通配符证书',
    desc: '一张证书覆盖所有子域名，部署更省心',
    url: topsslUrl('app-home', 'wildcard', '/ssl/wildcard'),
  },
  {
    title: '企业证书（OV/EV）',
    desc: '地址栏显示企业身份，客户更信任',
    url: topsslUrl('app-home', 'ov-ev', '/ssl/ov'),
  },
]

const counts = {
  all: () => certsStore.certs.length,
  valid: () => certsStore.certs.filter((c) => c.status === 'issued' && c.days_remaining > 30).length,
  expiring: () => certsStore.certs.filter((c) => c.status === 'issued' && c.days_remaining <= 30).length,
  expired: () => certsStore.certs.filter((c) => c.status === 'expired' || c.status === 'failed').length,
}
</script>

<template>
  <div class="mx-auto max-w-4xl px-6 py-8">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold text-slate-900">我的证书</h1>
        <p class="mt-0.5 text-sm text-slate-500">管理您申请到的 SSL 证书，到期前会自动续期</p>
      </div>
      <button class="btn-brand" @click="router.push('/wizard')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-4 w-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        一键申请证书
      </button>
    </div>

    <div class="mt-5 grid gap-3 sm:grid-cols-3">
      <a
        v-for="c in promoCards"
        :key="c.title"
        :href="c.url"
        class="group rounded-xl border border-brand-100 bg-white p-3.5 transition hover:border-brand-300 hover:shadow-sm"
        @click.prevent="openExternal(c.url)"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="text-sm font-semibold text-slate-800">{{ c.title }}</span>
          <span class="shrink-0 text-xs font-medium text-brand-600 group-hover:underline">了解详情 ›</span>
        </div>
        <p class="mt-1 text-xs leading-relaxed text-slate-500">{{ c.desc }}</p>
      </a>
    </div>

    <div class="mt-5 flex gap-1.5">
      <button
        v-for="(label, key) in { all: '全部', valid: '有效', expiring: '即将到期', expired: '已失效' }"
        :key="key"
        class="rounded-full px-3 py-1.5 text-xs font-medium transition"
        :class="filter === key ? 'bg-brand-600 text-white' : 'bg-white text-slate-600 border border-slate-200 hover:bg-slate-50'"
        @click="filter = key as typeof filter"
      >
        {{ label }} ({{ counts[key as keyof typeof counts]() }})
      </button>
    </div>

    <div v-if="certsStore.loading" class="mt-6 grid gap-4 sm:grid-cols-2">
      <div v-for="i in 4" :key="i" class="h-40 animate-pulse rounded-xl bg-slate-200/70"></div>
    </div>

    <div v-else-if="certsStore.certs.length" class="mt-6 grid gap-4 sm:grid-cols-2">
      <CertCard
        v-for="c in filteredCerts()"
        :key="c.id"
        :cert="c"
        @detail="(c) => (detailCert = c)"
        @renew="onRenew"
        @remove="onRemove"
      />
    </div>

    <div v-else class="mt-10 flex flex-col items-center rounded-2xl border border-dashed border-slate-300 bg-white py-14 text-center">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="h-12 w-12 text-slate-300">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
        />
      </svg>
      <h3 class="mt-4 text-base font-semibold text-slate-800">还没有证书</h3>
      <p class="mt-1 max-w-xs text-sm text-slate-500">输入您的域名，一分钟内获得免费的 Let's Encrypt 证书</p>
      <button class="btn-brand mt-5" @click="router.push('/wizard')">立即申请</button>
    </div>

    <CertDetailModal :cert="detailCert" @close="detailCert = null" />
    <ConfirmDialog
      ref="confirmRef"
      title="删除证书记录？"
      :message="`将删除 ${pendingRemove?.domain ?? ''} 的证书记录与相关文件，此操作不可恢复。`"
      confirm-text="删除"
      danger
    />
  </div>
</template>
