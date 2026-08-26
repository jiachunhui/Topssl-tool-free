<script setup lang="ts">
import { computed, ref, watch } from 'vue'

const props = defineProps<{
  domain: string
  altNames: string[]
  challengeType: 'http01' | 'dns01'
  directory: 'staging' | 'production'
  email: string
  submitting: boolean
}>()
const emit = defineEmits<{
  'update:directory': [v: 'staging' | 'production']
  'update:email': [v: string]
  next: []
  back: []
}>()

const email = ref(props.email)
const showProductionWarn = ref(false)

const emailValid = computed(() => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value.trim()))
// 有效性只由邮箱与「已选环境」决定：切到正式环境的确认弹窗本身已是一道门槛，
// 不再依赖本地 agreeProduction 标志（该标志在返回本页时会被重置，导致
// directory=production 时按钮误禁用，必须点一下环境才能激活）
const formValid = computed(() => emailValid.value)

watch(email, (v) => emit('update:email', v))

function toggleDirectory() {
  const next = props.directory === 'staging' ? 'production' : 'staging'
  if (next === 'production' && !props.directory) return
  if (next === 'production') {
    showProductionWarn.value = true
    return
  }
  emit('update:directory', 'staging')
}

function confirmProduction() {
  showProductionWarn.value = false
  emit('update:directory', 'production')
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-bold text-slate-900">确认申请信息</h2>
      <p class="mt-1 text-sm text-slate-500">请核对以下信息，然后开始申请</p>
    </div>

    <div class="rounded-xl border border-slate-200 bg-white p-4">
      <dl class="space-y-2.5 text-sm">
        <div class="flex justify-between gap-4">
          <dt class="shrink-0 text-slate-400">主域名</dt>
          <dd class="font-mono font-medium text-slate-800">{{ domain }}</dd>
        </div>
        <div v-if="altNames.length" class="flex justify-between gap-4">
          <dt class="shrink-0 text-slate-400">其他域名</dt>
          <dd class="font-mono font-medium text-slate-800">{{ altNames.join(', ') }}</dd>
        </div>
        <div class="flex justify-between gap-4">
          <dt class="shrink-0 text-slate-400">验证方式</dt>
          <dd class="font-medium text-slate-800">{{ challengeType === 'http01' ? 'HTTP 验证（80 端口）' : 'DNS 验证（TXT 记录）' }}</dd>
        </div>
      </dl>
    </div>

    <div>
      <label class="mb-1.5 block text-sm font-medium text-slate-700">接收邮箱（用于账户注册通知）</label>
      <input
        v-model="email"
        type="email"
        placeholder="例如：you@example.com"
        class="w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 text-sm text-slate-900 outline-none transition focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20"
      />
      <p v-if="email && !emailValid" class="mt-1.5 text-xs text-red-500">邮箱格式不正确</p>
    </div>

    <div
      class="cursor-pointer rounded-xl border-2 p-4 transition"
      :class="directory === 'production' ? 'border-brand-500 bg-brand-50/60' : 'border-slate-200 bg-white hover:border-slate-300'"
      @click="toggleDirectory"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm font-bold text-slate-900">
          {{ directory === 'staging' ? '测试环境（Staging）' : '正式环境（Production）' }}
        </span>
        <span v-if="directory === 'staging'" class="rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-medium text-amber-700">当前</span>
        <span v-else class="rounded-full bg-brand-600 px-2 py-0.5 text-[10px] font-medium text-white">正式</span>
      </div>
      <p class="mt-1.5 text-xs leading-relaxed text-slate-500">
        <template v-if="directory === 'staging'">测试证书不会被浏览器信任，仅用于验证流程。</template>
        <template v-else>正式证书由浏览器信任，注意 Let's Encrypt 速率限制。</template>
        点击切换 →
      </p>
    </div>

    <div class="flex justify-between pt-2">
      <button class="btn-secondary" @click="emit('back')">上一步</button>
      <button class="btn-brand" :disabled="!formValid || submitting" @click="emit('next')">
        {{ submitting ? '正在提交…' : '开始申请' }}
      </button>
    </div>

    <Teleport to="body">
      <div
        v-if="showProductionWarn"
        class="fixed inset-0 z-[90] flex items-center justify-center bg-slate-900/40 p-4"
        @click.self="showProductionWarn = false"
      >
        <div class="w-full max-w-sm rounded-xl bg-white p-5 shadow-xl fade-in">
          <h3 class="text-base font-semibold text-slate-900">切换到正式环境？</h3>
          <p class="mt-2 text-sm leading-relaxed text-slate-600">
            正式环境签发的证书可被浏览器信任，但同一域名每周最多签发 50 张，且申请失败会占用配额。建议先用测试环境验证流程。
          </p>
          <div class="mt-5 flex justify-end gap-2">
            <button class="btn-secondary" @click="showProductionWarn = false">取消</button>
            <button class="btn-brand" @click="confirmProduction">确认使用正式环境</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
