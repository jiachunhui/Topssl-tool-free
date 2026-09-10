<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { useCertsStore } from '../stores/certs'
import ConfirmDialog from '../components/ui/ConfirmDialog.vue'
import { api } from '../lib/api'
import { toast } from '../lib/toast'
import type { BackupImportResult } from '../lib/types'

const settingsStore = useSettingsStore()
const certsStore = useCertsStore()

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

// ---------- 数据备份与迁移 ----------

/** 与后端 backup::MIN_PASSWORD_LEN / MAX_BLOB_LEN 保持一致 */
const MIN_PASSWORD_LEN = 8
const MAX_BACKUP_BYTES = 64 * 1024 * 1024

type BackupMode = 'idle' | 'export' | 'import'
const mode = ref<BackupMode>('idle')

const exportPassword = ref('')
const exportPassword2 = ref('')
const exporting = ref(false)

const importInput = ref<HTMLInputElement | null>(null)
const importFile = ref<File | null>(null)
const importPassword = ref('')
const importing = ref(false)
const importResult = ref<BackupImportResult | null>(null)
const confirmRef = ref<InstanceType<typeof ConfirmDialog> | null>(null)

const backupBusy = () => exporting.value || importing.value

/** 去掉文件名，得到所在目录（Windows 与 POSIX 分隔符都处理） */
function dirnameOf(p: string): string {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i > 0 ? p.slice(0, i) : p
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

function toggleExport() {
  mode.value = mode.value === 'export' ? 'idle' : 'export'
  importResult.value = null
}

async function doExport() {
  if (exportPassword.value.length < MIN_PASSWORD_LEN) {
    toast.warn(`备份口令至少 ${MIN_PASSWORD_LEN} 位`)
    return
  }
  if (exportPassword.value !== exportPassword2.value) {
    toast.warn('两次输入的口令不一致')
    return
  }
  exporting.value = true
  try {
    const path = await api.exportBackupPackage(exportPassword.value)
    exportPassword.value = ''
    exportPassword2.value = ''
    mode.value = 'idle'
    toast.success('备份包已生成，正在打开所在文件夹')
    await api.openPath(dirnameOf(path)).catch(() => {})
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '导出失败')
  } finally {
    exporting.value = false
  }
}

function pickImportFile() {
  importInput.value?.click()
}

function onImportFileChange(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0] ?? null
  // 清空以便再次选择同一个文件
  input.value = ''
  if (!file) return
  if (file.size > MAX_BACKUP_BYTES) {
    toast.error('备份文件过大（上限 64 MB）')
    return
  }
  importFile.value = file
  importPassword.value = ''
  importResult.value = null
  mode.value = 'import'
}

async function doImport() {
  const file = importFile.value
  if (!file) return
  if (!importPassword.value) {
    toast.warn('请输入备份口令')
    return
  }
  const ok = await confirmRef.value?.ask()
  if (!ok) return

  importing.value = true
  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    const res = await api.importBackupPackage(importPassword.value, bytesToBase64(bytes))
    importResult.value = res
    importPassword.value = ''
    importFile.value = null
    mode.value = 'idle'
    toast.success(`导入完成：${res.certCount} 张证书`)
    await Promise.all([
      certsStore.fetchCerts().catch(() => {}),
      settingsStore.fetchSettings().catch(() => {}),
    ])
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '导入失败')
  } finally {
    importing.value = false
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
        <p class="mt-1 text-xs text-slate-400">
          证书有效期 90 天。开启后，应用运行时会在到期前 30 天自动续期；即使错过，应用每次启动也会补检并提醒到期/过期证书，可一键续期。个人电脑请同时开启「开机自动启动」和「证书到期提醒」。
        </p>
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
        <p class="mt-1 text-xs text-slate-400">RSA 兼容性好；ECC 更快更安全，但个别老旧平台仅支持 RSA</p>
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
        <p class="mt-1 text-xs text-slate-400">
          本机监听端口（默认 80）。仅当服务器 80 端口被 Web 服务占用时，改为高位端口并配合 80→该端口的反向代理使用；家庭宽带通常被运营商封锁 80 端口，个人电脑请直接用 DNS 验证。
        </p>
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

      <div class="rounded-2xl border border-slate-200 bg-white p-5">
        <h2 class="text-sm font-semibold text-slate-800">数据备份与迁移</h2>
        <p class="mt-1 text-xs leading-relaxed text-slate-400">
          换电脑时，把设置、证书、DNS 凭据与 ACME 账户密钥打包成一个加密文件，在新电脑导入即可继续使用。
          备份包用你设置的口令加密，<span class="font-medium text-amber-600">口令无法找回</span>，请妥善保管备份文件与口令。
        </p>
        <p class="mt-2 rounded-lg bg-slate-50 p-3 text-xs leading-relaxed text-slate-500">
          这是「迁移快照」而不是实时同步：导入得到的是导出那一刻的副本，之后两台电脑各自独立。
          为避免两台电脑重复为同一域名续期、互相干扰 DNS 校验，导入后会自动关闭自动续期。
        </p>

        <div class="mt-3 flex gap-2">
          <button class="btn-secondary !px-3 !py-1.5 text-xs" :disabled="backupBusy()" @click="toggleExport">
            导出备份包
          </button>
          <button class="btn-secondary !px-3 !py-1.5 text-xs" :disabled="backupBusy()" @click="pickImportFile">
            导入备份包
          </button>
        </div>

        <div v-if="mode === 'export'" class="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3">
          <p class="text-xs font-medium text-slate-600">设置备份口令（至少 8 位）</p>
          <div class="mt-2 flex flex-wrap gap-2">
            <input
              v-model="exportPassword"
              type="password"
              placeholder="备份口令"
              class="w-40 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm outline-none transition focus:border-brand-500"
            />
            <input
              v-model="exportPassword2"
              type="password"
              placeholder="再输入一次"
              class="w-40 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm outline-none transition focus:border-brand-500"
            />
            <button class="btn-brand !px-3 !py-1.5 text-xs" :disabled="exporting" @click="doExport">
              {{ exporting ? '打包中…' : '生成备份包' }}
            </button>
          </div>
          <p class="mt-2 text-xs text-slate-400">备份包会保存到系统「下载」文件夹。</p>
        </div>

        <div v-if="mode === 'import' && importFile" class="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3">
          <p class="break-all text-xs font-medium text-slate-600">{{ importFile.name }}</p>
          <div class="mt-2 flex flex-wrap gap-2">
            <input
              v-model="importPassword"
              type="password"
              placeholder="输入备份口令"
              class="w-40 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm outline-none transition focus:border-brand-500"
            />
            <button class="btn-brand !px-3 !py-1.5 text-xs" :disabled="importing" @click="doImport">
              {{ importing ? '导入中…' : '开始导入' }}
            </button>
            <button class="btn-secondary !px-3 !py-1.5 text-xs" :disabled="importing" @click="mode = 'idle'">
              取消
            </button>
          </div>
          <p class="mt-2 text-xs leading-relaxed text-amber-700">
            导入会覆盖本机现有的设置、证书与密钥（导入前会自动备份当前数据）。
          </p>
        </div>

        <div v-if="importResult" class="mt-3 rounded-lg border border-brand-200 bg-brand-50 p-3">
          <p class="text-xs font-semibold text-brand-800">导入完成</p>
          <ul class="mt-1.5 space-y-1 text-xs leading-relaxed text-brand-700">
            <li>
              已导入 {{ importResult.certCount }} 张证书、{{ importResult.providerCount }} 个 DNS 服务商、{{
                importResult.secretCount
              }} 条密钥
            </li>
            <li v-if="importResult.sourceHost">
              来源：{{ importResult.sourceHost }} · {{ importResult.createdAt.slice(0, 10) }}
            </li>
            <li v-if="importResult.missingFiles > 0" class="text-red-600">
              {{ importResult.missingFiles }} 张证书在备份包中缺少证书文件，请在「我的证书」中检查。
            </li>
            <li class="text-amber-700">自动续期已自动关闭，请只在一台常开的电脑上重新开启。</li>
          </ul>
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="api.openPath(importResult.safetyDir)">
              打开导入前的数据备份
            </button>
            <span class="break-all text-xs text-slate-400">{{ importResult.safetyDir }}</span>
          </div>
        </div>

        <input ref="importInput" type="file" class="hidden" @change="onImportFileChange" />
      </div>
    </div>

    <div class="mt-6 flex justify-end">
      <button class="btn-brand" :disabled="saving" @click="saveAll">{{ saving ? '保存中…' : '保存设置' }}</button>
    </div>

    <ConfirmDialog
      ref="confirmRef"
      title="导入备份包？"
      message="将用备份包中的内容覆盖本机的设置、证书记录与密钥。导入前会自动把当前数据备份到应用数据目录，但仍建议确认无误后再继续。"
      confirm-text="覆盖导入"
      danger
    />
  </div>

  <div v-else-if="settingsStore.error" class="mx-auto max-w-2xl px-6 py-8">
    <div class="rounded-2xl border border-red-200 bg-red-50 p-6">
      <h1 class="text-lg font-bold text-red-700">设置加载失败</h1>
      <p class="mt-2 break-all text-sm text-red-600">{{ settingsStore.error }}</p>
      <button class="btn-secondary mt-4" @click="settingsStore.fetchSettings()">重试</button>
    </div>
  </div>
</template>
