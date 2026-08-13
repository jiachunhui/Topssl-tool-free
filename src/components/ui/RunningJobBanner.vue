<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useJobStore } from '../../stores/job'

// 全局任务进度悬浮条：申请任务进行中且不在向导页时显示，点击返回向导继续操作
const route = useRoute()
const router = useRouter()
const jobStore = useJobStore()

const percent = () => jobStore.progress?.percent ?? jobStore.status?.percent ?? 0
const title = () => (jobStore.activeDomain ? `正在为 ${jobStore.activeDomain} 申请证书` : '证书任务进行中')
</script>

<template>
  <div v-if="jobStore.running && route.path !== '/wizard'" class="fixed left-1/2 top-4 z-[95] -translate-x-1/2">
    <button
      class="flex items-center gap-2 rounded-full border border-emerald-300 bg-emerald-50/95 px-4 py-2 text-sm font-medium text-emerald-800 shadow-lg backdrop-blur transition hover:bg-emerald-100"
      @click="router.push('/wizard')"
    >
      <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-emerald-500"></span>
      {{ title() }}（{{ percent() }}%）· 点击查看进度
    </button>
  </div>
</template>
