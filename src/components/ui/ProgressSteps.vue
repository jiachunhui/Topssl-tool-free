<script setup lang="ts">
defineProps<{
  steps: string[]
  current: number // 0-based
}>()
</script>

<template>
  <ol class="flex items-center gap-1">
    <li v-for="(s, i) in steps" :key="s" class="flex items-center gap-1">
      <template v-if="i > 0">
        <div class="mx-1 h-px w-5 bg-slate-300" :class="i <= current ? '!bg-emerald-500' : ''"></div>
      </template>
      <div
        class="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium"
        :class="
          i < current
            ? 'bg-emerald-100 text-emerald-700'
            : i === current
              ? 'bg-emerald-600 text-white'
              : 'bg-slate-100 text-slate-500'
        "
      >
        <span
          class="flex h-4 w-4 items-center justify-center rounded-full text-[10px]"
          :class="i === current ? 'bg-white/25' : ''"
        >
          <svg v-if="i < current" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" class="h-3 w-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
          </svg>
          <span v-else>{{ i + 1 }}</span>
        </span>
        {{ s }}
      </div>
    </li>
  </ol>
</template>
