<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  title: string
  message?: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}>()

const open = ref(false)
let resolveFn: ((ok: boolean) => void) | null = null

function ask(): Promise<boolean> {
  open.value = true
  return new Promise((resolve) => {
    resolveFn = resolve
  })
}

function confirm() {
  open.value = false
  resolveFn?.(true)
  resolveFn = null
}
function cancel() {
  open.value = false
  resolveFn?.(false)
  resolveFn = null
}

defineExpose({ ask })
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-[90] flex items-center justify-center bg-slate-900/40 p-4" @click.self="cancel">
      <div class="w-full max-w-sm rounded-xl bg-white p-5 shadow-xl fade-in">
        <h3 class="text-base font-semibold text-slate-900">{{ props.title }}</h3>
        <p v-if="props.message" class="mt-2 text-sm leading-relaxed text-slate-600">{{ props.message }}</p>
        <div class="mt-5 flex justify-end gap-2">
          <button class="btn-secondary" @click="cancel">{{ props.cancelText ?? '取消' }}</button>
          <button class="btn-brand" :class="props.danger ? '!bg-red-600 hover:!bg-red-700' : ''" @click="confirm">
            {{ props.confirmText ?? '确定' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
