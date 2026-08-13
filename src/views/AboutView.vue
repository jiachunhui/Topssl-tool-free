<script setup lang="ts">
import { useAppStore } from '../stores/app'

const appStore = useAppStore()

const faqs = [
  {
    q: '证书有效期多久？',
    a: 'Let\u2019s Encrypt 证书有效期为 90 天。本应用会在到期前 30 天自动为您续期（需保持应用后台运行，已默认开启开机自启）。',
  },
  {
    q: '什么是测试证书（Staging）？',
    a: '测试环境签发的证书不被浏览器信任，仅用于验证申请流程是否通畅，不会占用正式配额。确认流程无误后，请切换到正式环境申请。',
  },
  {
    q: 'HTTP 验证和 DNS 验证有什么区别？',
    a: 'HTTP 验证：自动在 80 端口临时开启验证服务，需要域名解析到本机公网 IP 且 80 端口公网可访问。DNS 验证：通过添加 TXT 解析记录验证，无需 80 端口，还支持通配符证书，但需要配置 DNS 服务商 API。',
  },
  {
    q: '通配符证书支持吗？',
    a: '支持。输入 *.example.com 即可申请通配符证书，一张证书覆盖主域名及所有子域名。通配符证书只能使用 DNS 验证。',
  },
  {
    q: '证书安装在哪里？',
    a: '证书保存在应用数据目录下的 certs 文件夹中（可在证书详情页查看并打开）。您可以将证书路径配置到自己的 HTTPS 服务中，详情页提供 nginx / Apache 的引用示例。',
  },
  {
    q: '证书是 RSA 还是 ECC？某些平台只支持 RSA 怎么办？',
    a: '本应用默认签发 RSA 2048 证书（兼容性最好，几乎所有平台都支持）。可在「设置 → 证书密钥类型」切换为 ECC（P-384）：速度更快、更安全，但个别老旧平台（旧版 Java、部分 VPN 网关等）仅支持 RSA，请按目标平台选择。',
  },
  {
    q: '申请失败常见原因？',
    a: '最常见的是：1) HTTP 验证时 80 端口公网不可达（建议改用 DNS 验证）；2) DNS 验证时 API 密钥权限不足；3) 域名解析未生效。错误提示会给出具体建议。',
  },
]
</script>

<template>
  <div class="mx-auto max-w-2xl px-6 py-8">
    <h1 class="text-xl font-bold text-slate-900">关于</h1>

    <div class="mt-6 rounded-2xl border border-slate-200 bg-white p-6 text-center">
      <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-xl bg-emerald-600 text-white">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-6 w-6">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
          />
        </svg>
      </div>
      <h2 class="mt-3 text-base font-bold text-slate-900">SSL 证书助手</h2>
      <p class="mt-0.5 text-xs text-slate-400">v{{ appStore.appInfo?.version ?? '—' }} · {{ appStore.appInfo?.platform ?? '' }} {{ appStore.appInfo?.arch ?? '' }}</p>
      <p class="mx-auto mt-3 max-w-md text-sm leading-relaxed text-slate-500">
        为您的域名免费申请 Let's Encrypt SSL 证书，安装到本机并自动续期。支持 HTTP 与 DNS 两种验证方式、通配符证书、多域名证书。
      </p>
    </div>

    <div class="mt-6 space-y-3">
      <h2 class="text-sm font-semibold text-slate-800">常见问题</h2>
      <details
        v-for="f in faqs"
        :key="f.q"
        class="group rounded-xl border border-slate-200 bg-white px-4 py-3 open:shadow-sm"
      >
        <summary class="cursor-pointer list-none text-sm font-medium text-slate-800 group-open:text-emerald-700">
          {{ f.q }}
        </summary>
        <p class="mt-2 text-sm leading-relaxed text-slate-500">{{ f.a }}</p>
      </details>
    </div>
  </div>
</template>
