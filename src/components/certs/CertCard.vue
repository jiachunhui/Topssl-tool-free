<script setup lang="ts">
import { computed } from 'vue'
import type { CertInfo } from '../../lib/types'

const props = defineProps<{
  cert: CertInfo
}>()

const emit = defineEmits<{ detail: [c: CertInfo]; renew: [c: CertInfo]; remove: [c: CertInfo] }>()

const statusMeta = computed(() => {
  const s = props.cert.status
  if (s === 'issued') {
    // 到期前 7 天升级为红色警戒，与系统通知的 7 天临界提醒呼应
    if (props.cert.days_remaining <= 7) {
      return { label: '即将到期', cls: 'bg-red-100 text-red-700 animate-pulse', bar: 'bg-red-500' }
    }
    return props.cert.days_remaining <= 30
      ? { label: '即将到期', cls: 'bg-amber-100 text-amber-700', bar: 'bg-amber-500' }
      : { label: '有效', cls: 'bg-brand-100 text-brand-700', bar: 'bg-brand-500' }
  }
  if (s === 'renewing') return { label: '续期中', cls: 'bg-sky-100 text-sky-700', bar: 'bg-sky-500' }
  if (s === 'failed') return { label: '失败', cls: 'bg-red-100 text-red-700', bar: 'bg-red-500' }
  if (s === 'expired') return { label: '已过期', cls: 'bg-red-100 text-red-700 animate-pulse', bar: 'bg-red-500' }
  return { label: '已吊销', cls: 'bg-slate-200 text-slate-600', bar: 'bg-slate-400' }
})

const remainingPct = computed(() => {
  if (props.cert.days_remaining <= 0) return 0
  return Math.min(100, Math.round((props.cert.days_remaining / 90) * 100))
})

const isStaging = computed(() => props.cert.directory === 'staging')
</script>

<template>
  <div
    class="group relative rounded-xl border border-slate-200 bg-white p-4 shadow-sm transition hover:shadow-md"
    :class="isStaging ? 'border-dashed' : ''"
    @click="emit('detail', cert)"
  >
    <div class="flex items-start justify-between gap-2">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="truncate font-mono text-sm font-semibold text-slate-900">{{ cert.domain }}</span>
          <span v-if="isStaging" class="shrink-0 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700"
            >测试证书</span
          >
        </div>
        <div v-if="cert.alt_names.length" class="mt-0.5 truncate font-mono text-[11px] text-slate-400">
          {{ cert.alt_names.join(' · ') }}
        </div>
      </div>
      <span class="shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium" :class="statusMeta.cls">
        {{ statusMeta.label }}
      </span>
    </div>

    <div class="mt-3">
      <div class="flex items-center justify-between text-[11px] text-slate-500">
        <span>剩余 {{ cert.days_remaining }} 天</span>
        <span>{{ cert.expires_at.slice(0, 10) }} 到期</span>
      </div>
      <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-slate-100">
        <div class="h-full rounded-full transition-all" :class="statusMeta.bar" :style="{ width: remainingPct + '%' }"></div>
      </div>
    </div>

    <div class="mt-3 flex items-center gap-1.5 text-[11px] text-slate-400">
      <span class="rounded bg-slate-100 px-1.5 py-0.5">{{ cert.challenge_type === 'http01' ? 'HTTP 验证' : 'DNS 验证' }}</span>
      <span class="rounded bg-slate-100 px-1.5 py-0.5">{{ cert.issuer ?? 'Let\u2019s Encrypt' }}</span>
    </div>

    <div class="mt-3 flex justify-end gap-1.5 border-t border-slate-100 pt-2.5 opacity-0 transition group-hover:opacity-100">
      <button class="rounded-md px-2 py-1 text-xs font-medium text-slate-600 hover:bg-slate-100" @click.stop="emit('detail', cert)">
        详情
      </button>
      <button
        v-if="cert.status !== 'renewing'"
        class="rounded-md px-2 py-1 text-xs font-medium text-brand-700 hover:bg-brand-50"
        @click.stop="emit('renew', cert)"
      >
        立即续期
      </button>
      <button class="rounded-md px-2 py-1 text-xs font-medium text-red-600 hover:bg-red-50" @click.stop="emit('remove', cert)">
        删除
      </button>
    </div>
  </div>
</template>
