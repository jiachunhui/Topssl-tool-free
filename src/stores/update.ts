// 应用更新状态机：检查（启动静默 / 关于页手动）→ 弹窗提示 → 下载（进度）→ 安装
// 「稍后提醒」的忽略版本由后端设置表持久化（dismiss_update），本 store 内存同步一份
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '../lib/api'
import { onUpdateProgress } from '../lib/events'
import type { UpdateInfo, UpdateProgress } from '../lib/types'

export type UpdatePhase =
  | 'idle' // 未检查
  | 'checking' // 检查中
  | 'up-to-date' // 已是最新
  | 'available' // 发现新版本（可下载 / 下载失败可重试）
  | 'downloading' // 下载中
  | 'downloaded' // 安装包已就绪
  | 'installing' // 正在启动安装程序
  | 'error' // 检查失败

export const useUpdateStore = defineStore('update', () => {
  const phase = ref<UpdatePhase>('idle')
  const info = ref<UpdateInfo | null>(null)
  const progress = ref<UpdateProgress>({ received: 0, total: 0 })
  const errorMessage = ref('')
  const installerPath = ref<string | null>(null)
  const dismissedVersion = ref<string | null>(null)
  /** 是否显示更新弹窗（「稍后」关闭；关于页手动检查会再次打开） */
  const promptVisible = ref(false)

  const percent = computed(() =>
    progress.value.total > 0
      ? Math.min(100, Math.round((progress.value.received / progress.value.total) * 100))
      : 0,
  )

  let unlistenProgress: (() => void) | null = null

  /** 启动初始化：读取已忽略版本 + 订阅下载进度（幂等） */
  async function init(): Promise<void> {
    if (unlistenProgress) return
    try {
      dismissedVersion.value = await api.getDismissedUpdateVersion()
    } catch {
      /* 忽略：读取失败按未忽略处理 */
    }
    unlistenProgress = (await onUpdateProgress((p) => {
      progress.value = p
    })).unlisten
  }

  /**
   * 检查更新。
   * force=false：自动检查，已忽略的版本不弹窗（但状态仍为 available，关于页可见）；
   * force=true：手动检查，强制联网，发现新版直接弹窗。
   */
  async function check(force = false): Promise<void> {
    phase.value = 'checking'
    errorMessage.value = ''
    try {
      const res = await api.checkUpdate(force)
      // 兜底：latestVersion 与当前版本相同时视为无更新（防旧缓存/异常数据误报）
      const available = res.available && res.latestVersion !== res.currentVersion
      info.value = { ...res, available }
      if (available) {
        phase.value = 'available'
        if (force || dismissedVersion.value !== res.latestVersion) {
          promptVisible.value = true
        }
      } else {
        phase.value = 'up-to-date'
      }
    } catch (e) {
      phase.value = 'error'
      errorMessage.value = e instanceof Error ? e.message : String(e)
      throw e
    }
  }

  /** 「稍后提醒」：忽略当前版本（后端持久化），关闭弹窗 */
  async function dismiss(): Promise<void> {
    const v = info.value?.latestVersion
    if (v) {
      dismissedVersion.value = v
      try {
        await api.dismissUpdate(v)
      } catch {
        /* 忽略：持久化失败仅影响下次启动 */
      }
    }
    promptVisible.value = false
  }

  function closePrompt(): void {
    promptVisible.value = false
  }

  /** 下载安装包（进度经 update://progress 事件推送） */
  async function download(): Promise<void> {
    if (!info.value?.available) return
    phase.value = 'downloading'
    progress.value = { received: 0, total: 0 }
    errorMessage.value = ''
    try {
      const path = await api.downloadUpdate()
      installerPath.value = path
      phase.value = 'downloaded'
    } catch (e) {
      const code = (e as { code?: string }).code
      if (code === 'ERR_CANCELED') {
        // 用户主动取消：安静回到可更新状态
        phase.value = 'available'
        errorMessage.value = ''
      } else {
        phase.value = 'available'
        errorMessage.value = e instanceof Error ? e.message : String(e)
      }
    }
  }

  /** 取消进行中的下载（后端中止并清理临时文件） */
  async function cancelDownload(): Promise<void> {
    try {
      await api.cancelUpdateDownload()
    } catch {
      /* 忽略 */
    }
  }

  /** 启动安装程序（Windows：退出应用后由 NSIS 覆盖安装） */
  async function install(): Promise<void> {
    if (!installerPath.value) return
    phase.value = 'installing'
    errorMessage.value = ''
    try {
      await api.installUpdate(installerPath.value)
    } catch (e) {
      phase.value = 'downloaded'
      errorMessage.value = e instanceof Error ? e.message : String(e)
    }
  }

  return {
    phase,
    info,
    progress,
    percent,
    errorMessage,
    installerPath,
    dismissedVersion,
    promptVisible,
    init,
    check,
    dismiss,
    closePrompt,
    download,
    cancelDownload,
    install,
  }
})
