<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { api } from '../lib/api'
import { toast } from '../lib/toast'

const settingsStore = useSettingsStore()

const saving = ref(false)
const privilegeNote = ref<string | null>(null)

onMounted(async () => {
  await settingsStore.fetchSettings()
  const platform = await api.getPlatformInfo().catch(() => null)
  privilegeNote.value = platform?.http01PrivilegeNote ?? null
})

async function saveAll() {
  saving.value = true
  try {
    await settingsStore.saveAll()
    toast.success('设置已保存')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '保存失败')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="mx-auto max-w-2xl px-6 py-8" v-if="settingsStore.loaded">    <h1 class="text-xl font-bold text-slate-900">设置</h1>

    <div class="mt-6 space-y-4">
      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">申请环境</h2>
        <p class="mt-1 text-xs text-slate-400">默认使用测试环境验证流程，正式环境受 Let's Encrypt 速率限制</p>
        <div class="mt-3 flex gap-2">
          <button
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition"
            :class="settingsStore.settings.acme_directory === 'staging' ? 'bg-amber-100 text-amber-700' : 'bg-slate-100 text-slate-500'"
            @click="settingsStore.setKey('acme_directory', 'staging')"
          >
            测试环境（Staging）
          </button>
          <button
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition"
            :class="settingsStore.settings.acme_directory === 'production' ? 'bg-brand-100 text-brand-700' : 'bg-slate-100 text-slate-500'"
            @click="settingsStore.setKey('acme_directory', 'production')"
          >
            正式环境（Production）
          </button>
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">邮箱</h2>
        <p class="mt-1 text-xs text-slate-400">用于 ACME 账户注册与到期通知</p>
        <input
          v-model="settingsStore.settings.contact_email"
          type="email"
          placeholder="you@example.com"
          class="mt-3 w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 text-sm outline-none transition focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20"
        />
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">自动续期</h2>
        <p class="mt-1 text-xs text-slate-400">证书有效期 90 天，开启后将在到期前 30 天自动续期（需应用保持后台运行）</p>
        <label class="mt-3 flex cursor-pointer items-center justify-between">
          <span class="text-sm text-slate-700">开启自动续期</span>
          <input
            type="checkbox"
            class="h-4 w-4 accent-brand-600"
            :checked="settingsStore.settings.auto_renew"
            @change="settingsStore.setKey('auto_renew', (settingsStore.settings.auto_renew = !settingsStore.settings.auto_renew))"
          />
        </label>
        <label class="mt-3 flex cursor-pointer items-center justify-between">
          <span class="text-sm text-slate-700">开机自动启动</span>
          <input
            type="checkbox"
            class="h-4 w-4 accent-brand-600"
            :checked="settingsStore.settings.run_at_login"
            @change="settingsStore.setKey('run_at_login', (settingsStore.settings.run_at_login = !settingsStore.settings.run_at_login))"
          />
        </label>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">系统通知</h2>
        <p class="mt-1 text-xs text-slate-400">证书到期与续期结果将通过系统通知和应用内提示告知</p>
        <label class="mt-3 flex cursor-pointer items-center justify-between">
          <span class="text-sm text-slate-700">证书到期提醒</span>
          <input
            type="checkbox"
            class="h-4 w-4 accent-brand-600"
            :checked="settingsStore.settings.notify_expiring"
            @change="settingsStore.setKey('notify_expiring', (settingsStore.settings.notify_expiring = !settingsStore.settings.notify_expiring))"
          />
        </label>
        <label class="mt-3 flex cursor-pointer items-center justify-between">
          <span class="text-sm text-slate-700">续期成功提醒</span>
          <input
            type="checkbox"
            class="h-4 w-4 accent-brand-600"
            :checked="settingsStore.settings.notify_renew_success"
            @change="settingsStore.setKey('notify_renew_success', (settingsStore.settings.notify_renew_success = !settingsStore.settings.notify_renew_success))"
          />
        </label>
        <label class="mt-3 flex cursor-pointer items-center justify-between">
          <span class="text-sm text-slate-700">续期失败提醒</span>
          <input
            type="checkbox"
            class="h-4 w-4 accent-brand-600"
            :checked="settingsStore.settings.notify_renew_failed"
            @change="settingsStore.setKey('notify_renew_failed', (settingsStore.settings.notify_renew_failed = !settingsStore.settings.notify_renew_failed))"
          />
        </label>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">证书密钥类型</h2>
        <p class="mt-1 text-xs text-slate-400">RSA 兼容性最好；ECC 更快更安全，但个别老旧平台仅支持 RSA</p>
        <div class="mt-3 flex gap-2">
          <button
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition"
            :class="settingsStore.settings.cert_key_type !== 'ecc' ? 'bg-brand-100 text-brand-700' : 'bg-slate-100 text-slate-500'"
            @click="settingsStore.setKey('cert_key_type', 'rsa')"
          >
            RSA（推荐）
          </button>
          <button
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition"
            :class="settingsStore.settings.cert_key_type === 'ecc' ? 'bg-brand-100 text-brand-700' : 'bg-slate-100 text-slate-500'"
            @click="settingsStore.setKey('cert_key_type', 'ecc')"
          >
            ECC（P-384）
          </button>
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">HTTP 验证端口</h2>
        <p class="mt-1 text-xs text-slate-400">HTTP-01 验证需要监听 80 端口（一般无需修改）</p>
        <input
          v-model.number="settingsStore.settings.http01_port"
          type="number"
          min="1"
          max="65535"
          class="mt-3 w-32 rounded-lg border border-slate-300 bg-white px-3.5 py-2 text-sm outline-none transition focus:border-brand-500"
        />
        <p
          v-if="settingsStore.settings.http01_port !== 80"
          class="mt-2 rounded-lg bg-amber-50 p-2.5 text-xs leading-relaxed text-amber-700"
        >
          注意：Let's Encrypt 的 HTTP-01 验证始终访问 80 端口。当前端口非 80 时，请确保 80 端口已反向代理/转发到该端口，否则验证必然失败（B7）。
        </p>
        <div v-if="privilegeNote" class="mt-3 rounded-lg bg-amber-50 p-3 text-xs leading-relaxed text-amber-700">
          {{ privilegeNote }}
        </div>
      </div>
    </div>

    <div class="mt-6 flex justify-end">
      <button class="btn-brand" :disabled="saving" @click="saveAll">{{ saving ? '保存中…' : '保存设置' }}</button>
    </div>
  </div>

  <div v-else-if="settingsStore.error" class="mx-auto max-w-2xl px-6 py-8">
    <div class="rounded-2xl border border-red-200 bg-red-50 p-6">
      <h1 class="text-lg font-bold text-red-700">设置加载失败</h1>
      <p class="mt-2 break-all text-sm text-red-600">{{ settingsStore.error }}</p>
      <button class="btn-secondary mt-4" @click="settingsStore.fetchSettings()">重试</button>
    </div>
  </div>
</template>
