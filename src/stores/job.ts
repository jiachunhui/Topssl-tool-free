import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '../lib/api'
import { onJobFinished, onJobProgress, onTxtNeeded } from '../lib/events'
import type { IssueRequest, JobProgress, JobStatus, TxtRecord } from '../lib/types'

export const useJobStore = defineStore('job', () => {
  const activeJobId = ref<string | null>(null)
  const progress = ref<JobProgress | null>(null)
  const status = ref<JobStatus | null>(null)
  const running = ref(false)
  const subscribed = ref(false)
  /** DNS 手动模式：待用户添加的 TXT 记录（可能多条，同一记录名下不同值） */
  const pendingTxts = ref<TxtRecord[]>([])
  /** 最近完成任务签发的证书 id（job-finished 事件携带） */
  const finishedCertId = ref<number | null>(null)
  /** 当前任务的目标域名（全局悬浮条展示用） */
  const activeDomain = ref<string | null>(null)

  /** 启动申请任务，订阅事件 */
  async function start(req: IssueRequest): Promise<string> {
    ensureSubscriptions()
    const jobId = await api.startIssue(req)
    activeJobId.value = jobId
    activeDomain.value = req.domain
    running.value = true
    progress.value = null
    status.value = null
    pendingTxts.value = []
    finishedCertId.value = null
    return jobId
  }

  async function cancel() {
    if (activeJobId.value) {
      await api.cancelIssue(activeJobId.value)
    }
  }

  /** 用户确认已手动添加全部 TXT 记录 */
  async function confirmTxtAdded() {
    if (activeJobId.value && pendingTxts.value.length) {
      await api.confirmTxt(activeJobId.value)
      pendingTxts.value = []
    }
  }

  /** 恢复/查询任务状态（页面刷新后或续期后）
   *  domain: 可选，用于悬浮条展示目标域名（续期场景由调用方传入）
   */
  async function restore(jobId: string, domain?: string) {
    ensureSubscriptions()
    const st = await api.getJobStatus(jobId)
    if (st) {
      activeJobId.value = jobId
      status.value = st
      running.value = st.state === 'running' || st.state === 'pending'
      if (domain) {
        activeDomain.value = domain
      }
    }
  }

  function ensureSubscriptions() {
    if (subscribed.value) return
    subscribed.value = true
    onJobProgress((p) => {
      // 仅接收当前任务的进度（当前无任务时兜底接收），避免多任务并存时进度串台
      if (p.job_id === activeJobId.value || activeJobId.value === null) {
        progress.value = p
      }
    })
    onJobFinished((p) => {
      // 完成事件：匹配当前任务；若当前无任务也接受（防止事件竞态导致卡死）
      if (p.job_id === activeJobId.value || activeJobId.value === null) {
        running.value = false
        pendingTxts.value = []
        status.value = {
          job_id: p.job_id,
          state: p.state ?? (p.ok ? 'completed' : 'failed'),
          stage: p.state === 'completed' ? 'Completed' : null,
          percent: p.state === 'completed' ? 100 : progress.value?.percent ?? 0,
          message: null,
          error_code: p.error_code ?? null,
          error_detail: p.error_detail ?? null,
          cert_id: p.cert_id ?? null,
        }
        finishedCertId.value = p.cert_id ?? null
        activeJobId.value = null
        activeDomain.value = null
      }
    })
    onTxtNeeded((t) => {
      if (t.jobId === activeJobId.value) {
        pendingTxts.value = t.records
      }
    })
  }

  function reset() {
    activeJobId.value = null
    progress.value = null
    status.value = null
    running.value = false
    pendingTxts.value = []
    finishedCertId.value = null
    activeDomain.value = null
  }

  return {
    activeJobId,
    progress,
    status,
    running,
    pendingTxts,
    finishedCertId,
    activeDomain,
    start,
    cancel,
    confirmTxtAdded,
    restore,
    reset,
  }
})
