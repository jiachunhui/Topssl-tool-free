<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { api } from '../../lib/api'

const props = defineProps<{
  domain: string
  altNames: string[]
  directory: 'staging' | 'production'
}>()
const emit = defineEmits<{
  update: [domain: string, altNames: string[]]
  next: []
}>()

const domain = ref(props.domain)
const altNamesText = ref(props.altNames.join(', '))
const checking = ref(false)
const duplicate = ref<{ duplicate: boolean; certId?: number } | null>(null)

// 支持 Unicode 字母/数字（IDN，与后端 idna 转 punycode 一致，轻微问题 3），
// 其余校验（标签长度、TLD 规则、非法字符）交给后端兜底
const DOMAIN_RE = /^(?:\*\.)?(?:[\p{L}\p{N}](?:[\p{L}\p{N}-]*[\p{L}\p{N}])?\.)+[\p{L}]{2,63}$/iu

const domainValid = computed(() => DOMAIN_RE.test(domain.value.trim()))
const altNamesList = computed(() =>
  altNamesText.value
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean),
)
const isWildcard = computed(() => domain.value.trim().startsWith('*.'))
/** 通配符主域下，被覆盖的一级子域 SAN（冗余，LE 会拒绝） */
const wildcardBase = computed(() => (isWildcard.value ? domain.value.trim().slice(2) : ''))
const redundantAlt = computed(() => {
  if (!wildcardBase.value) return null
  return (
    altNamesList.value.find((d) => {
      if (!d.endsWith('.' + wildcardBase.value)) return false
      const prefix = d.slice(0, d.length - wildcardBase.value.length - 1)
      return prefix.length > 0 && !prefix.includes('.')
    }) ?? null
  )
})
const altNamesValid = computed(
  () =>
    altNamesList.value.every((d) => DOMAIN_RE.test(d) && !d.startsWith('*.')) &&
    !redundantAlt.value,
)
const formValid = computed(() => domainValid.value && altNamesValid.value)

watch(
  () => [domain.value, altNamesText.value] as const,
  () => {
    duplicate.value = null
    emit('update', domain.value.trim(), altNamesList.value)
  },
)

async function checkDuplicate() {
  if (!domainValid.value) return
  checking.value = true
  try {
    duplicate.value = await api.checkDuplicate(domain.value.trim(), props.directory)
  } finally {
    checking.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-bold text-slate-900">申请哪个域名？</h2>
      <p class="mt-1 text-sm text-slate-500">填写您希望为它配置 HTTPS 的域名</p>
    </div>

    <div>
      <label class="mb-1.5 block text-sm font-medium text-slate-700">主域名</label>
      <input
        v-model="domain"
        type="text"
        placeholder="例如：example.com 或 *.example.com"
        class="w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 font-mono text-sm text-slate-900 outline-none transition focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20"
        @blur="checkDuplicate"
      />
      <div class="mt-1.5 flex items-start gap-1.5 text-xs text-slate-400">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" class="mt-0.5 h-3.5 w-3.5 shrink-0">
          <path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" />
        </svg>
        <span>
          <template v-if="isWildcard">通配符证书将覆盖主域名及所有子域名，只能使用 DNS 验证方式</template>
          <template v-else>以 *. 开头的为通配符证书；普通域名可同时支持 HTTP 与 DNS 验证</template>
        </span>
      </div>
      <p v-if="domain && !domainValid" class="mt-1.5 text-xs text-red-500">域名格式不正确</p>
      <div v-if="checking" class="mt-1.5 text-xs text-slate-400">检查中…</div>
      <div v-else-if="duplicate?.duplicate" class="mt-1.5 rounded-md bg-amber-50 px-3 py-1.5 text-xs text-amber-700">
        该域名已有有效证书（<RouterLink :to="'/'">查看详情</RouterLink>），可直接对现有证书续期
      </div>
    </div>

    <div>
      <label class="mb-1.5 block text-sm font-medium text-slate-700">其他域名（可选，多域名证书）</label>
      <input
        v-model="altNamesText"
        type="text"
        placeholder="用逗号分隔，例如：www.example.com, api.example.com"
        class="w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 font-mono text-sm text-slate-900 outline-none transition focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20"
      />
      <p v-if="redundantAlt" class="mt-1.5 text-xs text-red-500">
        {{ redundantAlt }} 已被通配符 *.{{ wildcardBase }} 覆盖，无需重复添加
      </p>
      <p v-else-if="altNamesList.length && !altNamesValid" class="mt-1.5 text-xs text-red-500">其他域名格式不正确（不支持通配符）</p>
    </div>

    <div class="flex justify-end pt-2">
      <button class="btn-brand" :disabled="!formValid" @click="emit('next')">下一步</button>
    </div>
  </div>
</template>
