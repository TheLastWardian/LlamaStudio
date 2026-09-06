import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { appConfig } from './config'
import { allModels, type ModelFile } from './selectedModel'
import { t } from '../i18n'

// Tipos wire (camelCase por serde) — espejo de downloads.rs (S2/S4)
export type JobState = 'queued' | 'downloading' | 'paused' | 'completed' | 'failed'

export interface TaskView {
  taskId: string
  fileName: string
  pathInRepo: string
  totalBytes: number
  downloadedBytes: number
  group: string
  state: JobState
  error: string | null
  retryCount: number
  // S5: código estable (rate_limited | not_found | gated | disk_full |
  // sha_mismatch | network | cancelled) — null si no hay error
  errorCode: string | null
  // S5: countdown de backoff en segundos; -1 = sin retry pendiente
  retryInSec: number
  // solo los llena el evento download-progress (no existen en JobView de Rust)
  speedBps?: number
  etaSec?: number
}

export interface JobView {
  jobId: string
  repoOwner: string
  repoName: string
  files: TaskView[]
  createdAt: number
  state: JobState
}

interface ProgressPayload {
  jobId: string
  taskId: string
  downloadedBytes: number
  totalBytes: number
  speedBps: number
  etaSec: number
}

interface StatePayload {
  jobId: string
  state: JobState
  error?: string | null
  // S5: Some(n) mientras una task del job espera backoff (Rust emite 1/s)
  retryInSec?: number | null
}

export interface FileSpecArg {
  path: string
  size: number
  sha256: string | null
  group: string
}

export const jobs = ref<JobView[]>([])

let inited = false

function findTask(jobId: string, taskId: string): TaskView | undefined {
  return jobs.value.find(j => j.jobId === jobId)?.files.find(f => f.taskId === taskId)
}

function upsertJob(view: JobView): void {
  const i = jobs.value.findIndex(j => j.jobId === view.jobId)
  if (i === -1) jobs.value = [view, ...jobs.value]
  else jobs.value[i] = view
}

// §6.2: job completado → re-escanear la librería para que los archivos nuevos
// (y el badge "en librería" del panel) aparezcan sin cambiar de vista
async function rescanModels(): Promise<void> {
  const modelsPath = appConfig.value.modelsPath
  if (!modelsPath) return
  try {
    allModels.value = await invoke<ModelFile[]>('scan_models', { modelsPath })
  } catch {
    // ModelsView re-escanea al mostrarse; no romper la barra
  }
}

function onState(payload: StatePayload): void {
  const job = jobs.value.find(j => j.jobId === payload.jobId)
  if (!job) {
    void refresh().catch(() => {})
    return
  }
  job.state = payload.state
  if (payload.error) {
    const t = job.files.find(f => f.state !== 'completed')
    if (t) t.error = payload.error
  }
  // S5: countdown del backoff (la task en espera queda 'queued')
  if (payload.retryInSec != null && payload.retryInSec > 0) {
    const retryTask = job.files.find(f => f.state === 'queued')
    if (retryTask) retryTask.retryInSec = payload.retryInSec
  } else {
    for (const f of job.files) f.retryInSec = -1
  }
  if (payload.state === 'completed') void rescanModels()
}

// S5: el código viene de Rust (stable, snake_case) y el texto visible se i18niza acá.
// `error` queda como detalle diagnóstico (fallback).
export function errorCodeMsg(code: string | null, fallback?: string | null): string {
  if (code) return t('downloads.errors.' + code)
  return fallback ?? ''
}

export function firstErrorMsg(job: JobView): string {
  const f = job.files.find(f => f.errorCode || f.error)
  if (!f) return ''
  return errorCodeMsg(f.errorCode, f.error)
}

// Se suscribe UNA sola vez (llamado desde App.vue onMounted). También carga los
// jobs persistidos de la sesión anterior (recovery de S2: .part → paused, etc.).
export async function init(): Promise<void> {
  if (inited) return
  inited = true

  await listen<ProgressPayload>('download-progress', (event) => {
    const p = event.payload
    const task = findTask(p.jobId, p.taskId)
    if (task) {
      task.downloadedBytes = p.downloadedBytes
      task.totalBytes = p.totalBytes
      task.speedBps = p.speedBps
      task.etaSec = p.etaSec
    }
  })

  await listen<StatePayload>('download-state', (event) => {
    onState(event.payload)
  })

  await refresh().catch(() => {})
}

// Acciones (todas = invokes; el backend re-deriva y emite el estado)
export function startDownloads(args: { modelsPath: string; owner: string; repo: string; files: FileSpecArg[] }): Promise<JobView> {
  const d = appConfig.value.downloads
  return invoke<JobView>('start_downloads', {
    ...args,
    parallelism: d.parallelism,
    autoRetry: d.autoRetry,
    maxRetries: d.maxRetries,
    keepPartOnCancel: d.keepPartOnCancel,
  }).then(job => {
    upsertJob(job)
    return job
  })
}

export function pause(jobId: string): Promise<JobView> {
  return invoke<JobView>('pause_download', { jobId }).then(job => { upsertJob(job); return job })
}

export function resume(jobId: string): Promise<JobView> {
  const d = appConfig.value.downloads
  return invoke<JobView>('resume_download', {
    jobId,
    modelsPath: appConfig.value.modelsPath,
    parallelism: d.parallelism,
    autoRetry: d.autoRetry,
    maxRetries: d.maxRetries,
    keepPartOnCancel: d.keepPartOnCancel,
  }).then(job => { upsertJob(job); return job })
}

export function cancel(jobId: string): Promise<JobView> {
  return invoke<JobView>('cancel_download', { jobId }).then(job => { upsertJob(job); return job })
}

export function remove(jobId: string): Promise<void> {
  return invoke('remove_download', { jobId }).then(() => {
    jobs.value = jobs.value.filter(j => j.jobId !== jobId)
  })
}

async function refresh(): Promise<void> {
  jobs.value = await invoke<JobView[]>('list_downloads', { modelsPath: appConfig.value.modelsPath })
}

// Helpers de agregado para la barra
export interface JobTotals {
  doneBytes: number
  totalBytes: number
  speedBps: number
  activeCount: number
}

export function jobTotals(job: JobView): JobTotals {
  let doneBytes = 0
  let totalBytes = 0
  let speedBps = 0
  let activeCount = 0
  for (const f of job.files) {
    totalBytes += f.totalBytes
    doneBytes += f.state === 'completed' ? f.totalBytes : f.downloadedBytes
    if (f.state === 'downloading') {
      speedBps += f.speedBps ?? 0
      activeCount++
    }
  }
  return { doneBytes, totalBytes, speedBps, activeCount }
}
