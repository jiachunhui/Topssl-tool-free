<script setup lang="ts">
import { topsslUrl, openExternal } from '../../lib/promo'

const navs = [
  { to: '/', label: '我的证书', icon: 'M3.75 9.75l8.25-6.75 8.25 6.75M4.5 9v9.75a.75.75 0 0 0 .75.75h13.5a.75.75 0 0 0 .75-.75V9' },
  {
    to: '/wizard',
    label: '申请证书',
    icon: 'M12 4.5v15m7.5-7.5h-15',
  },
  {
    to: '/dns',
    label: 'DNS 配置',
    icon: 'M5.25 8.25h13.5M5.25 12h13.5M5.25 15.75h13.5M4.5 19.5h15a.75.75 0 0 0 .75-.75V5.25a.75.75 0 0 0-.75-.75h-15a.75.75 0 0 0-.75.75v13.5a.75.75 0 0 0 .75.75Z',
  },
  {
    to: '/settings',
    label: '设置',
    icon: 'M10.343 3.94c.09-.542.56-.94 1.11-.94h1.093c.55 0 1.02.398 1.11.94l.149.894c.07.424.384.764.78.93.398.164.855.142 1.205-.108l.737-.527a1.125 1.125 0 0 1 1.45.12l.773.774c.39.389.44 1.002.12 1.45l-.527.737c-.25.35-.272.806-.107 1.204.165.397.505.71.93.78l.893.15c.543.09.94.56.94 1.109v1.094c0 .55-.397 1.02-.94 1.11l-.893.149c-.425.07-.765.383-.93.78-.165.398-.143.854.107 1.204l.527.738c.32.447.269 1.06-.12 1.45l-.774.773a1.125 1.125 0 0 1-1.449.12l-.738-.527c-.35-.25-.806-.272-1.203-.107-.397.165-.71.505-.781.929l-.149.894c-.09.542-.56.94-1.11.94h-1.094c-.55 0-1.019-.398-1.11-.94l-.148-.894c-.071-.424-.384-.764-.781-.93-.398-.164-.854-.142-1.204.108l-.738.527c-.447.32-1.06.269-1.45-.12l-.773-.774a1.125 1.125 0 0 1-.12-1.45l.527-.737c.25-.35.273-.806.108-1.204-.165-.397-.505-.71-.93-.78l-.894-.15c-.542-.09-.94-.56-.94-1.109v-1.094c0-.55.398-1.02.94-1.11l.894-.149c.424-.07.765-.383.93-.78.165-.398.143-.854-.108-1.204l-.526-.738a1.125 1.125 0 0 1 .12-1.45l.773-.773a1.125 1.125 0 0 1 1.45-.12l.737.527c.35.25.807.272 1.204.107.397-.165.71-.505.78-.929l.15-.894Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  },
  {
    to: '/logs',
    label: '应用日志',
    icon: 'M7.5 8.25h9m-9 3.75h9m-9 3.75h4.5M19.5 4.5h-15a.75.75 0 0 0-.75.75v13.5a.75.75 0 0 0 .75.75h15a.75.75 0 0 0 .75-.75V5.25a.75.75 0 0 0-.75-.75Z',
  },
  { to: '/about', label: '关于', icon: 'M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z' },
]
</script>

<template>
  <aside class="flex w-52 shrink-0 flex-col border-r border-slate-200 bg-white">
    <div class="flex items-center gap-2.5 px-5 pt-5 pb-4">
      <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-brand-600 text-white">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-5 w-5">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
          />
        </svg>
      </div>
      <div>
        <div class="text-sm leading-tight font-bold text-slate-900">
          <span class="block">TopSSL</span>
          <span class="block">免费证书助手</span>
        </div>
        <div class="mt-0.5 text-[11px] text-slate-400">Let's Encrypt 免费证书</div>
      </div>
    </div>
    <nav class="flex flex-col gap-1 px-3 pt-2">
      <RouterLink
        v-for="n in navs"
        :key="n.to"
        :to="n.to"
        class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-slate-600 transition hover:bg-slate-100 hover:text-slate-900"
        active-class="!bg-brand-50 !text-brand-700"
      >
        <svg class="h-[18px] w-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7">
          <path stroke-linecap="round" stroke-linejoin="round" :d="n.icon" />
        </svg>
        {{ n.label }}
      </RouterLink>
      <button
        class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-slate-600 transition hover:bg-slate-100 hover:text-slate-900"
        title="访问 TopSSL 官网"
        @click="openExternal(topsslUrl('app-sidebar', 'home'))"
      >
        <svg class="h-[18px] w-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M13.5 6H5.25A2.25 2.25 0 0 0 3 8.25v10.5A2.25 2.25 0 0 0 5.25 21h10.5A2.25 2.25 0 0 0 18 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25"
          />
        </svg>
        TopSSL
        <svg class="ml-auto h-3.5 w-3.5 text-slate-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
          <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 6H5.25A2.25 2.25 0 0 0 3 8.25v10.5A2.25 2.25 0 0 0 5.25 21h10.5A2.25 2.25 0 0 0 18 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
        </svg>
      </button>
    </nav>
    <div class="mt-auto px-5 pb-4 text-[11px] leading-relaxed text-slate-400">
      证书有效期 90 天，本应用会自动在到期前 30 天为您续期
    </div>
  </aside>
</template>
