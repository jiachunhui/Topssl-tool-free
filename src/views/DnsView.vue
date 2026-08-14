<script setup lang="ts">
import { onMounted, ref } from 'vue'
import ConfirmDialog from '../components/ui/ConfirmDialog.vue'
import { useProvidersStore } from '../stores/providers'
import { toast } from '../lib/toast'
import type { ProviderInfo, ProviderInput } from '../lib/types'

const providersStore = useProvidersStore()
const confirmRef = ref<InstanceType<typeof ConfirmDialog> | null>(null)
const pendingDelete = ref<ProviderInfo | null>(null)

const editing = ref(false)
const form = ref<ProviderInput>({ kind: 'aliyun', label: '', config: {} })
const formErrors = ref<string[]>([])
const testingId = ref<number | null>(null)
const testResult = ref<{ ok: boolean; message: string } | null>(null)

const KIND_META: Record<string, { name: string; fields: { key: string; label: string; secret?: boolean; placeholder?: string }[] }> = {
  aliyun: {
    name: '阿里云 DNS',
    fields: [
      { key: 'access_key_id', label: 'AccessKey ID', placeholder: 'LTAI5t...' },
      { key: 'access_key_secret', label: 'AccessKey Secret', secret: true },
    ],
  },
  dnspod: {
    name: 'DNSPod / 腾讯云',
    fields: [
      { key: 'token_id', label: 'Token ID（DNSPod 旧版可不填）', placeholder: '12345' },
      { key: 'login_token', label: 'API Token', secret: true, placeholder: 'ID,Token' },
    ],
  },
  cloudflare: {
    name: 'Cloudflare',
    fields: [
      { key: 'api_token', label: 'API Token（Zone:DNS:Edit 权限）', secret: true, placeholder: 'xxxxxxxx' },
    ],
  },
}

onMounted(() => providersStore.fetchProviders())

function startCreate(kind: 'aliyun' | 'dnspod' | 'cloudflare') {
  form.value = { kind, label: KIND_META[kind].name, config: {} }
  formErrors.value = []
  testResult.value = null
  editing.value = true
}

function startEdit(p: ProviderInfo) {
  form.value = {
    id: p.id,
    kind: p.kind,
    label: p.label,
    config: { ...p.config },
  }
  formErrors.value = []
  testResult.value = null
  editing.value = true
}

async function save() {
  formErrors.value = []
  const errs: string[] = []
  if (!form.value.label.trim()) errs.push('请填写名称')
  // 新建时必须填写密钥；编辑时密钥可选（留空保留原密钥，后端支持，M3）
  const isEdit = !!form.value.id
  for (const f of KIND_META[form.value.kind].fields) {
    const v = form.value.config[f.key]?.trim()
    if (!v && f.secret && !isEdit) errs.push(`请填写 ${f.label}`)
  }
  if (errs.length) {
    formErrors.value = errs
    return
  }
  try {
    // 编辑时留空的密钥字段不上传，避免后端以空串覆盖旧密钥
    const SECRET_FIELDS = new Set(['access_key_secret', 'login_token', 'api_token'])
    const cleanConfig = Object.fromEntries(
      Object.entries(form.value.config)
        .map(([k, v]) => [k, String(v ?? '').trim()])
        .filter(([k, v]) => !(SECRET_FIELDS.has(k) && v === '')),
    )
    await providersStore.save({ ...form.value, config: cleanConfig })
    toast.success('已保存')
    editing.value = false
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '保存失败')
  }
}

async function testProvider(id: number) {
  testingId.value = id
  testResult.value = null
  try {
    const r = await providersStore.test(id)
    testResult.value = r
    toast[r.ok ? 'success' : 'error'](r.message)
  } catch (e) {
    testResult.value = { ok: false, message: e instanceof Error ? e.message : '测试失败' }
  } finally {
    testingId.value = null
  }
}

async function removeProvider(p: ProviderInfo) {
  pendingDelete.value = p
  const ok = await confirmRef.value?.ask()
  if (!ok) return
  try {
    await providersStore.remove(p.id)
    toast.success('已删除')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '删除失败')
  }
}
</script>

<template>
  <div class="mx-auto max-w-3xl px-6 py-8">
    <div>
      <h1 class="text-xl font-bold text-slate-900">DNS 服务商配置</h1>
      <p class="mt-0.5 text-sm text-slate-500">
        使用 DNS 验证（DNS-01）时需要配置服务商 API，用于自动添加/删除验证记录。支持通配符证书申请。
      </p>
    </div>

    <div v-if="!editing" class="mt-6 space-y-3">
      <div v-if="!providersStore.providers.length" class="rounded-xl border border-dashed border-slate-300 bg-white p-6 text-center text-sm text-slate-500">
        尚未配置任何服务商，点击下方按钮添加
      </div>
      <div
        v-for="p in providersStore.providers"
        :key="p.id"
        class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-4 py-3"
      >
        <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-slate-100 text-sm font-bold text-slate-600">
          {{ p.kind === 'aliyun' ? '阿' : p.kind === 'dnspod' ? '腾' : 'CF' }}
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold text-slate-800">{{ p.label }}</span>
            <span class="rounded bg-slate-100 px-1.5 py-0.5 text-[11px] text-slate-500">{{ KIND_META[p.kind].name }}</span>
          </div>
          <div class="text-xs text-slate-400">已保存 {{ p.created_at.slice(0, 10) }}</div>
        </div>
        <button class="btn-secondary !px-2.5 !py-1.5 text-xs" :disabled="testingId === p.id" @click="testProvider(p.id)">
          {{ testingId === p.id ? '测试中…' : '测试' }}
        </button>
        <button class="btn-secondary !px-2.5 !py-1.5 text-xs" @click="startEdit(p)">编辑</button>
        <button class="btn-danger !px-2.5 !py-1.5 text-xs" @click="removeProvider(p)">删除</button>
      </div>

      <div v-if="testResult" class="rounded-lg p-3 text-sm" :class="testResult.ok ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'">
        {{ testResult.message }}
      </div>

      <div class="grid gap-3 pt-2 sm:grid-cols-3">
        <button
          v-for="(meta, kind) in KIND_META"
          :key="kind"
          class="rounded-xl border-2 border-dashed border-slate-300 bg-white px-4 py-4 text-left transition hover:border-emerald-400 hover:bg-emerald-50/40"
          @click="startCreate(kind as 'aliyun' | 'dnspod' | 'cloudflare')"
        >
          <div class="text-sm font-bold text-slate-800">+ 添加{{ meta.name }}</div>
          <div class="mt-1 text-xs text-slate-400">使用 API 自动管理 TXT 记录</div>
        </button>
      </div>
    </div>

    <div v-else class="mt-6 rounded-2xl border border-slate-200 bg-white p-6">
      <h2 class="text-base font-semibold text-slate-900">{{ form.id ? '编辑' : '添加' }}{{ KIND_META[form.kind].name }}</h2>
      <p v-if="form.id" class="mt-1 text-xs text-slate-400">编辑时密钥留空将保留原值；切换服务商类型需重新填写新类型的密钥。</p>
      <div class="mt-4 space-y-4">
        <div>
          <label class="mb-1.5 block text-sm font-medium text-slate-700">名称</label>
          <input v-model="form.label" type="text" placeholder="例如：我的阿里云" class="input-base w-full" />
        </div>
        <div v-for="f in KIND_META[form.kind].fields" :key="f.key">
          <label class="mb-1.5 block text-sm font-medium text-slate-700">{{ f.label }}</label>
          <input
            v-model="form.config[f.key]"
            :type="f.secret ? 'password' : 'text'"
            :placeholder="f.placeholder ?? ''"
            class="w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 font-mono text-sm outline-none transition focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/20"
            autocomplete="off"
          />
        </div>
        <p v-if="form.kind === 'dnspod'" class="text-xs text-slate-400">
          提示：DNSPod 旧版使用 API Token（ID,Token）；腾讯云新用户请使用 API 密钥（SecretId/SecretKey）可留空此页，稍后在申请时选择腾讯云签名方式。当前版本支持 DNSPod 旧版 Token。
        </p>
        <div v-if="formErrors.length" class="rounded-lg bg-red-50 p-3 text-sm text-red-600">
          <p v-for="e in formErrors" :key="e">{{ e }}</p>
        </div>
      </div>
      <div class="mt-5 flex justify-end gap-2">
        <button class="btn-secondary" @click="editing = false">取消</button>
        <button class="btn-brand" @click="save">保存</button>
      </div>
    </div>

    <ConfirmDialog
      ref="confirmRef"
      title="删除服务商配置？"
      :message="`将删除「${pendingDelete?.label ?? ''}」的配置。若仍有证书正在使用该服务商，将无法删除。`"
      confirm-text="删除"
      danger
    />
  </div>
</template>

<style scoped>
.input-base {
  border-radius: 0.5rem;
  border: 1px solid var(--color-slate-300);
  background: white;
  padding: 0.625rem 0.875rem;
  font-size: 0.875rem;
  outline: none;
}
.input-base:focus {
  border-color: var(--color-emerald-500);
  box-shadow: 0 0 0 2px rgb(16 185 129 / 0.2);
}
</style>
