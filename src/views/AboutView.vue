<script setup lang="ts">
import { useAppStore } from '../stores/app'
import { useUpdateStore } from '../stores/update'
import { topsslUrl, openExternal, GITHUB_REPO } from '../lib/promo'

const appStore = useAppStore()
const updateStore = useUpdateStore()

/** 手动检查更新：发现新版直接弹窗（错误文案由 store 记录并展示） */
async function checkUpdate() {
  try {
    await updateStore.check(true)
  } catch {
    /* 状态与文案已在 store 中 */
  }
}

const faqs = [
  {
    q: '证书有效期多久？',
    a: 'Let\u2019s Encrypt 证书有效期为 90 天。开启自动续期并保持应用运行时，会在到期前 30 天自动续期；即使错过，应用每次启动也会补检并提醒到期/过期证书，可一键续期。个人电脑建议开启「开机自启」与「到期提醒」。',
  },
  {
    q: '什么是测试证书（Staging）？',
    a: '测试环境签发的证书不被浏览器信任，仅用于验证申请流程是否通畅，不会占用正式配额。确认流程无误后，请切换到正式环境申请。',
  },
  {
    q: 'HTTP 验证和 DNS 验证有什么区别？',
    a: 'HTTP 验证：自动在 80 端口临时开启验证服务，需要域名解析到本机公网 IP 且 80 端口公网可访问（家庭宽带通常被运营商封锁 80 端口，个人电脑建议直接用 DNS 验证）。DNS 验证：通过添加 TXT 解析记录验证，无需 80 端口，还支持通配符证书，但需要配置 DNS 服务商 API。',
  },
  {
    q: '通配符证书支持吗？',
    a: '支持。输入 *.example.com 即可申请通配符证书，一张证书覆盖主域名及所有子域名。通配符证书只能使用 DNS 验证。',
  },
  {
    q: '证书安装在哪里？',
    a: '证书保存在应用数据目录下的 certs 文件夹中（可在证书详情页查看并打开）。您可以将证书路径配置到自己的 HTTPS 服务中，详情页提供 nginx / Apache 的引用示例；也可以一键导出「部署包」（证书文件 + 各平台配置示例）拷贝到服务器使用，Windows 服务器还支持 IIS 一键部署。',
  },
  {
    q: '证书怎么部署到我的服务器？',
    a: '在证书详情页点击「导出部署包」，应用会生成一个包含 fullchain.pem、privkey.pem 与 nginx/Apache/IIS 配置示例的文件夹，拷贝到服务器按说明配置即可。Windows 服务器可直接使用「IIS 一键部署」（需以管理员身份运行 Tossl），自动导入证书并绑定 https。',
  },
  {
    q: '证书是 RSA 还是 ECC？某些平台只支持 RSA 怎么办？',
    a: '本应用默认签发 RSA 2048 证书（兼容性最好，几乎所有平台都支持）。可在「设置 → 证书密钥类型」切换为 ECC（P-384）：速度更快、更安全，但个别老旧平台（旧版 Java、部分 VPN 网关等）仅支持 RSA，请按目标平台选择。',
  },
  {
    q: '申请失败常见原因？',
    a: '最常见的是：1) HTTP 验证时 80 端口公网不可达（建议改用 DNS 验证）；2) DNS 验证时 API 密钥权限不足；3) 域名解析未生效。错误提示会给出具体建议。',
  },
  {
    q: 'TopSSL 是什么？',
    a: 'TopSSL（topssl.cn）是专业的 SSL 证书服务平台，提供企业级 SSL 证书（DV / OV / EV）、证书自动化部署方案与专业技术支持。本应用由 TopSSL 出品并支持，免费工具部分完全开源。',
  },
  {
    q: '免费证书和企业证书有什么区别？',
    a: '免费证书（本应用）有效期 90 天，到期自动续期，适合个人网站与测试用途；企业证书（OV/EV）地址栏直接显示企业身份，客户信任度更高，可购买多年期省去频繁续期，且兼容更老旧的设备与系统。',
  },
  {
    q: 'TopSSL 平台提供自动化部署方案吗？',
    a: '提供。TopSSL 平台支持证书在 CDN、云服务器、负载均衡等场景的自动化部署与更新方案，适合批量证书管理的企业用户，可访问官网了解详情。',
  },
  {
    q: '遇到申请或部署问题怎么办？',
    a: '免费工具遇到问题可先查看应用内错误提示、用户手册或开源仓库提交 Issue；需要即时人工协助时，TopSSL 平台提供专业技术支持服务（免费工具申请渠道所不具备的）。',
  },
]
</script>

<template>
  <div class="mx-auto max-w-2xl px-6 py-8">
    <h1 class="text-xl font-bold text-slate-900">关于</h1>

    <div class="mt-6 rounded-2xl border border-slate-200 bg-white p-6 text-center">
      <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-xl bg-brand-600 text-white">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-6 w-6">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
          />
        </svg>
      </div>
      <h2 class="mt-3 text-base font-bold text-slate-900">Tossl 免费SSL证书管理工具</h2>
      <p class="mt-0.5 text-xs text-slate-400">v{{ appStore.appInfo?.version ?? '—' }} · {{ appStore.appInfo?.platform ?? '' }} {{ appStore.appInfo?.arch ?? '' }}</p>
      <div class="mt-2.5 flex items-center justify-center gap-2 text-xs">
        <span v-if="updateStore.phase === 'checking'" class="text-slate-400">正在检查更新…</span>
        <span v-else-if="updateStore.phase === 'error'" class="text-red-500">{{ updateStore.errorMessage || '检查更新失败' }}</span>
        <span v-else-if="updateStore.info?.available" class="font-medium text-orange-600">
          发现新版本 v{{ updateStore.info.latestVersion }}
        </span>
        <span v-else-if="updateStore.phase === 'up-to-date'" class="text-emerald-600">已是最新版本</span>
        <button
          class="btn-secondary !px-2.5 !py-1 text-xs"
          :disabled="updateStore.phase === 'checking'"
          @click="checkUpdate"
        >
          检查更新
        </button>
      </div>
      <p class="mx-auto mt-3 max-w-md text-sm leading-relaxed text-slate-500">
        为您的域名免费申请 Let's Encrypt SSL 证书，安装到本机并自动续期。支持 HTTP 与 DNS 两种验证方式、通配符证书、多域名证书。
      </p>
      <p class="mt-3 text-xs text-slate-400">
        技术支持由
        <button class="font-medium text-brand-700 underline underline-offset-2 hover:text-brand-800" @click="openExternal(topsslUrl('app-about', 'home'))">
          TopSSL（topssl.cn）
        </button>
        提供
      </p>
    </div>

    <div class="mt-6 rounded-xl border border-slate-200 bg-white px-4 py-3.5">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="text-sm font-semibold text-slate-800">开源项目</div>
          <p class="mt-0.5 text-xs text-slate-500">本项目基于 MIT 协议开源，最新版本与更新记录见 GitHub 仓库</p>
        </div>
        <button
          class="btn-secondary !px-3 !py-1.5 text-xs"
          @click="openExternal(GITHUB_REPO)"
        >
          <svg viewBox="0 0 24 24" fill="currentColor" class="mr-1 inline h-3.5 w-3.5 align-[-2px]">
            <path d="M12 2C6.48 2 2 6.58 2 12.25c0 4.53 2.87 8.37 6.84 9.73.5.1.68-.22.68-.49 0-.24-.01-.88-.01-1.73-2.78.62-3.37-1.37-3.37-1.37-.45-1.18-1.11-1.5-1.11-1.5-.91-.63.07-.62.07-.62 1 .07 1.53 1.06 1.53 1.06.89 1.57 2.34 1.12 2.91.85.09-.66.35-1.12.63-1.37-2.22-.26-4.56-1.14-4.56-5.07 0-1.12.39-2.03 1.03-2.75-.1-.26-.45-1.3.1-2.7 0 0 .84-.28 2.75 1.05a9.36 9.36 0 0 1 5 0c1.91-1.33 2.75-1.05 2.75-1.05.55 1.4.2 2.44.1 2.7.64.72 1.03 1.63 1.03 2.75 0 3.94-2.34 4.8-4.57 5.06.36.32.68.94.68 1.9 0 1.37-.01 2.47-.01 2.81 0 .27.18.6.69.49A10.25 10.25 0 0 0 22 12.25C22 6.58 17.52 2 12 2Z" />
          </svg>
          GitHub
        </button>
      </div>
    </div>

    <div class="mt-6 space-y-3">
      <h2 class="text-sm font-semibold text-slate-800">常见问题</h2>
      <details
        v-for="f in faqs"
        :key="f.q"
        class="group rounded-xl border border-slate-200 bg-white px-4 py-3 open:shadow-sm"
      >
        <summary class="cursor-pointer list-none text-sm font-medium text-slate-800 group-open:text-brand-700">
          {{ f.q }}
        </summary>
        <p class="mt-2 text-sm leading-relaxed text-slate-500">{{ f.a }}</p>
      </details>
    </div>
  </div>
</template>
