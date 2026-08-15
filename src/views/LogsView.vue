<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { api } from '../lib/api'
import { toast } from '../lib/toast'
import type { LogEntry } from '../lib/types'

const logs = ref<LogEntry[]>([])
const loading = ref(false)
const levelFilter = ref<'all' | 'INFO' | 'WARN' | 'ERROR'>('all')
const autoRefresh = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

const levelColor: Record<string, string> = {
  INFO: 'text-sky-600',
  WARN: 'text-amber-600',
  ERROR: 'text-red-600',
}

async function refresh() {
  loading.value = true
  try {
    logs.value = await api.getLogs(500)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '读取日志失败')
  } finally {
    loading.value = false
  }
}

const visibleLogs = () => {
  if (levelFilter.value === 'all') return logs.value
  return logs.value.filter((l) => l.level === levelFilter.value)
}

async function copyAll() {
  const text = logs.value.map((l) => `[${l.time}][${l.level}] ${l.msg}`).join('\n')
  try {
    await api.copyToClipboard(text)
    toast.success('日志已复制，可直接粘贴反馈')
  } catch {
    toast.error('复制失败')
  }
}

async function clear() {
  try {
    await api.clearLogs()
    logs.value = []
    toast.success('日志已清空')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : '清空失败')
  }
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value
  if (autoRefresh.value) {
    timer = setInterval(refresh, 3000)
  } else if (timer) {
    clearInterval(timer)
    timer = null
  }
}

onMounted(refresh)

onUnmounted(() => {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
})
</script>

<template>
  <div class="mx-auto max-w-4xl px-6 py-8">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold text-slate-900">应用日志</h1>
        <p class="mt-0.5 text-sm text-slate-500">
          日志用于排查问题。遇到错误时，点击"复制日志"后粘贴发给我们即可。
        </p>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="toggleAutoRefresh">
          {{ autoRefresh ? '自动刷新：开' : '自动刷新：关' }}
        </button>
        <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="refresh">刷新</button>
        <button class="btn-secondary !px-3 !py-1.5 text-xs" @click="clear">清空</button>
        <button class="btn-brand !px-3 !py-1.5 text-xs" @click="copyAll">复制日志</button>
      </div>
    </div>

    <div class="mt-4 flex gap-1.5">
      <button
        v-for="(label, key) in { all: '全部', INFO: '信息', WARN: '警告', ERROR: '错误' }"
        :key="key"
        class="rounded-full px-3 py-1 text-xs font-medium transition"
        :class="levelFilter === key ? 'bg-brand-600 text-white' : 'bg-white text-slate-600 border border-slate-200'"
        @click="levelFilter = key as typeof levelFilter"
      >
        {{ label }}
      </button>
    </div>

    <div class="mt-4 overflow-hidden rounded-xl border border-slate-200 bg-white">
      <div v-if="loading && !logs.length" class="p-6 text-center text-sm text-slate-400">加载中…</div>
      <div v-else-if="!visibleLogs().length" class="p-6 text-center text-sm text-slate-400">暂无日志</div>
      <div v-else class="max-h-[62vh] overflow-y-auto p-2 font-mono text-xs leading-relaxed">
        <div
          v-for="(l, i) in visibleLogs()"
          :key="i"
          class="flex gap-2 rounded px-2 py-1 hover:bg-slate-50"
        >
          <span class="shrink-0 text-slate-400">{{ l.time }}</span>
          <span class="w-12 shrink-0 font-bold" :class="levelColor[l.level] ?? 'text-slate-500'">{{ l.level }}</span>
          <span class="break-all text-slate-700">{{ l.msg }}</span>
        </div>
      </div>
    </div>

    <p class="mt-3 text-xs text-slate-400">
      日志同时保存在应用数据目录的 app.log 文件中。此页面为开发调试工具，正式发布时可移除。
    </p>
  </div>
</template>
