<script setup lang="ts">
// 更新提示弹窗：发现新版 → 下载（进度条）→ 启动安装
import { computed } from 'vue'
import { useUpdateStore } from '../../stores/update'
import { openExternal } from '../../lib/promo'

const store = useUpdateStore()

const visible = computed(
  () => store.promptVisible && ['available', 'downloading', 'downloaded', 'installing'].includes(store.phase),
)

const sizeText = computed(() => {
  const n = store.info?.asset?.size ?? 0
  if (n >= 1048576) return (n / 1048576).toFixed(1) + ' MB'
  if (n > 0) return Math.round(n / 1024) + ' KB'
  return ''
})

const receivedText = computed(() => {
  const n = store.progress.received
  return n >= 1048576 ? (n / 1048576).toFixed(1) + ' MB' : Math.round(n / 1024) + ' KB'
})

function openGitHub(): void {
  const url = store.info?.releasePage
  if (url) openExternal(url)
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4"
      @click.self="store.phase !== 'downloading' && store.phase !== 'installing' && store.closePrompt()"
    >
      <div class="fade-in w-full max-w-md rounded-2xl border border-slate-200 bg-white p-6 shadow-2xl">
        <!-- 标题 -->
        <div class="flex items-start gap-3">
          <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-brand-100 text-brand-700">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-5 w-5">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M3 15.5 8.5 10l3.5 3.5 4-4 5 6M3 19h18M4 4h16a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Z"
              />
            </svg>
          </div>
          <div class="min-w-0">
            <h2 v-if="store.phase === 'available'" class="text-base font-bold text-slate-900">
              发现新版本 v{{ store.info?.latestVersion }}
            </h2>
            <h2 v-else-if="store.phase === 'downloading'" class="text-base font-bold text-slate-900">正在下载更新…</h2>
            <h2 v-else-if="store.phase === 'downloaded'" class="text-base font-bold text-slate-900">安装包已就绪</h2>
            <h2 v-else class="text-base font-bold text-slate-900">正在启动安装程序…</h2>
            <p class="mt-0.5 text-xs text-slate-400">
              当前版本 v{{ store.info?.currentVersion ?? '—' }} ·
              {{ store.info?.source === 'domestic' ? '国内更新源' : 'GitHub Releases' }}
            </p>
          </div>
          <button
            v-if="store.phase === 'available' || store.phase === 'downloaded'"
            class="ml-auto rounded-lg p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-600"
            title="关闭"
            @click="store.closePrompt()"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-4 w-4">
              <path stroke-linecap="round" d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>

        <!-- 发现新版本：更新说明 + 操作 -->
        <template v-if="store.phase === 'available'">
          <div
            v-if="store.info?.notes"
            class="mt-4 max-h-44 overflow-y-auto whitespace-pre-wrap rounded-lg bg-slate-50 px-3 py-2.5 text-xs leading-relaxed text-slate-600"
          >
            {{ store.info.notes }}
          </div>
          <p v-if="store.info?.asset" class="mt-3 text-xs text-slate-400">
            安装包：{{ store.info.asset.name }}<span v-if="sizeText">（{{ sizeText }}）</span>
          </p>
          <p
            v-else
            class="mt-3 rounded-lg bg-amber-50 px-3 py-2 text-xs leading-relaxed text-amber-700"
          >
            当前平台暂未提供自动更新安装包，请前往 GitHub Releases 手动下载。
          </p>
          <p v-if="store.errorMessage" class="mt-2 text-xs text-red-500">{{ store.errorMessage }}</p>
          <div class="mt-5 flex items-center justify-between gap-2">
            <button class="text-xs font-medium text-slate-400 underline underline-offset-2 hover:text-slate-600" @click="openGitHub">
              前往 GitHub 查看
            </button>
            <div class="flex gap-2">
              <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="store.dismiss()">稍后提醒</button>
              <button class="btn-brand !px-3 !py-1.5 text-xs" @click="store.download()">
                {{ store.info?.asset ? '立即更新' : '下载安装包' }}
              </button>
            </div>
          </div>
        </template>

        <!-- 下载中：进度条 -->
        <template v-else-if="store.phase === 'downloading'">
          <div class="mt-5">
            <div class="h-2 w-full overflow-hidden rounded-full bg-slate-100">
              <div
                class="h-full rounded-full bg-brand-600 transition-all duration-200"
                :style="{ width: (store.progress.total > 0 ? store.percent : 8) + '%' }"
              ></div>
            </div>
            <div class="mt-2 flex items-center justify-between text-xs text-slate-400">
              <span>
                {{ receivedText }} / {{ sizeText || '…' }}
                <span v-if="store.progress.total > 0">（{{ store.percent }}%）</span>
              </span>
              <button class="font-medium text-slate-500 underline underline-offset-2 hover:text-slate-700" @click="store.cancelDownload()">
                取消
              </button>
            </div>
          </div>
        </template>

        <!-- 下载完成：安装 -->
        <template v-else-if="store.phase === 'downloaded'">
          <p class="mt-4 text-sm leading-relaxed text-slate-600">
            新版本 v{{ store.info?.latestVersion }} 的安装包已下载完成<span v-if="sizeText">（{{ sizeText }}）</span>。
            点击「安装并重启」将退出当前应用并启动安装程序。
          </p>
          <p v-if="store.errorMessage" class="mt-2 text-xs text-red-500">{{ store.errorMessage }}</p>
          <div class="mt-5 flex justify-end gap-2">
            <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="store.closePrompt()">暂不安装</button>
            <button class="btn-brand !px-3 !py-1.5 text-xs" @click="store.install()">安装并重启</button>
          </div>
        </template>

        <!-- 启动安装中 -->
        <template v-else>
          <p class="mt-4 text-sm text-slate-500">正在启动安装程序，应用即将退出…</p>
        </template>
      </div>
    </div>
  </Teleport>
</template>
