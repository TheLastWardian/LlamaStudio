<template>
  <div v-if="jobs.length" class="dl-bar">
    <div class="dl-header" @click="collapsed = !collapsed">
      <span class="dl-dot" :class="dotClass"></span>
      <span class="dl-title">{{ t('downloads.title') }}</span>
      <span class="dl-summary">
        {{ t('downloads.active', { n: nActive }) }}
        <span class="dl-sep">·</span>
        {{ t('downloads.completed', { n: nCompleted }) }}
        <span class="dl-sep">·</span>
        {{ t('downloads.failed', { n: nFailed }) }}
      </span>
      <span class="dl-caret">{{ collapsed ? '▸' : '▾' }}</span>
    </div>

    <div v-show="!collapsed" class="dl-list">
      <div v-for="r in rows" :key="r.job.jobId" class="dl-row">
        <div class="dl-row-main">
          <div class="dl-row-top">
            <span class="dl-repo">{{ r.job.repoOwner }}/{{ r.job.repoName }}</span>
            <span class="dl-files">{{ t('downloads.files', { n: r.job.files.length }) }}</span>
            <span class="dl-glyph" :class="'glyph-' + r.job.state" :title="t('downloads.state.' + r.job.state)">{{ glyph(r.job.state) }}</span>
          </div>
          <div class="dl-track">
            <div class="dl-fill"
                 :class="r.job.state === 'failed' ? 'fill-failed' : r.job.state === 'completed' ? 'fill-done' : 'fill-active'"
                 :style="{ width: r.pct + '%' }"></div>
          </div>
          <div class="dl-meta">
            {{ fmtSize(r.doneBytes) }} / {{ fmtSize(r.totalBytes) }}
            <template v-if="r.job.state === 'downloading'">
              · {{ fmtSpeed(r.speedBps) }}
              <template v-if="r.etaSec !== null"> · {{ t('downloads.eta', { time: fmtEta(r.etaSec) }) }}</template>
            </template>
            <template v-if="r.retryIn > 0"> · {{ t('downloads.retryIn', { n: r.retryIn }) }}</template>
            <template v-if="r.job.state === 'failed'"> · {{ firstErrorMsg(r.job) }}</template>
          </div>
        </div>
        <div class="dl-actions" @click.stop>
          <button v-if="r.job.state === 'downloading' || r.job.state === 'queued'" class="dl-btn" :title="t('downloads.pause')" @click="onPauseResume(r.job)">
            <Pause :size="14" />
          </button>
          <button v-else-if="r.job.state === 'paused'" class="dl-btn" :title="t('downloads.resume')" @click="onPauseResume(r.job)">
            <Play :size="14" />
          </button>
          <button v-if="r.job.state !== 'completed' && r.job.state !== 'failed'" class="dl-btn" :title="t('downloads.cancel')" @click="onCancel(r.job)">
            <X :size="14" />
          </button>
          <button class="dl-btn" :title="t('downloads.openFolder')" @click="onOpenFolder(r.job)">
            <FolderOpen :size="14" />
          </button>
          <button v-if="r.job.state === 'completed' || r.job.state === 'failed'" class="dl-btn" :title="t('downloads.remove')" @click="onRemove(r.job)">
            <Trash2 :size="14" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { openPath } from '@tauri-apps/plugin-opener'
import { Pause, Play, X, FolderOpen, Trash2 } from '@lucide/vue'
import { t } from '../i18n'
import { appConfig } from '../stores/config'
import { jobs, jobTotals, pause, resume, cancel, remove, firstErrorMsg, type JobView, type JobState } from '../stores/downloads'

const collapsed = ref(false)

interface Row {
  job: JobView
  doneBytes: number
  totalBytes: number
  speedBps: number
  activeCount: number
  pct: number
  etaSec: number | null
  // S5: countdown mayor de la job (0 = sin backoff pendiente)
  retryIn: number
}

const rows = computed<Row[]>(() =>
  jobs.value.map(job => {
    const tot = jobTotals(job)
    const pct = tot.totalBytes > 0 ? Math.min(100, (tot.doneBytes / tot.totalBytes) * 100) : 0
    const etas = job.files
      .filter(f => f.state === 'downloading')
      .map(f => f.etaSec)
      .filter((e): e is number => typeof e === 'number' && e >= 0)
    const retries = job.files.filter(f => f.retryInSec > 0).map(f => f.retryInSec)
    return { job, ...tot, pct, etaSec: etas.length ? Math.max(...etas) : null, retryIn: retries.length ? Math.max(...retries) : 0 }
  }))

const nActive = computed(() => jobs.value.filter(j => j.state === 'downloading' || j.state === 'queued').length)
const nCompleted = computed(() => jobs.value.filter(j => j.state === 'completed').length)
const nFailed = computed(() => jobs.value.filter(j => j.state === 'failed').length)

const dotClass = computed(() => {
  if (nActive.value > 0) return 'dot-active'
  if (nFailed.value > 0) return 'dot-error'
  return 'dot-idle'
})

function glyph(s: JobState): string {
  switch (s) {
    case 'downloading': return '▼'
    case 'paused': return '❚❚'
    case 'queued': return '…'
    case 'completed': return '✓'
    case 'failed': return '✕'
  }
}

function fmtSize(n: number): string {
  if (n >= 1024 ** 3) return (n / 1024 ** 3).toFixed(1) + ' GB'
  if (n >= 1024 ** 2) return Math.round(n / 1024 ** 2) + ' MB'
  return Math.max(1, Math.round(n / 1024)) + ' KB'
}

function fmtSpeed(bps: number): string {
  if (bps >= 1024 * 1024) return (bps / (1024 * 1024)).toFixed(1) + ' MB/s'
  return Math.max(1, Math.round(bps / 1024)) + ' KB/s'
}

function fmtEta(sec: number): string {
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  if (m >= 60) return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}`
  return `${m}:${String(s).padStart(2, '0')}`
}

function onPauseResume(job: JobView): void {
  if (job.state === 'downloading' || job.state === 'queued') void pause(job.jobId).catch(() => {})
  else if (job.state === 'paused') void resume(job.jobId).catch(() => {})
}

function onCancel(job: JobView): void {
  void cancel(job.jobId).catch(() => {})
}

function onRemove(job: JobView): void {
  void remove(job.jobId).catch(() => {})
}

function onOpenFolder(job: JobView): void {
  const base = appConfig.value.modelsPath
  if (!base) return
  void openPath(`${base}/${job.repoOwner}/${job.repoName}`).catch(() => {})
}
</script>

<style scoped>
.dl-bar {
  background: #1a1a1a;
  border-top: 1px solid #2a2a2a;
  display: flex;
  flex-direction: column;
  flex: 0 0 auto;
}

.dl-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  user-select: none;
  flex: 0 0 auto;
}

.dl-header:hover { background: #1e1e1e; }

.dl-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #666;
  flex: 0 0 auto;
}

.dot-active { background: #4af54a; }
.dot-error { background: #f55a5a; }
.dot-idle { background: #666; }

.dl-title {
  font-size: 12px;
  font-weight: 600;
  color: #d4d4d4;
}

.dl-summary {
  font-size: 12px;
  color: #888;
}

.dl-sep { color: #444; margin: 0 4px; }

.dl-caret {
  margin-left: auto;
  color: #666;
  font-size: 12px;
}

.dl-list {
  overflow-y: auto;
  max-height: 220px;
  border-top: 1px solid #2a2a2a;
}

.dl-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid #2a2a2a;
}

.dl-row:last-child { border-bottom: none; }

.dl-row-main { flex: 1; min-width: 0; }

.dl-row-top {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.dl-repo {
  font-size: 12px;
  font-weight: 600;
  color: #d4d4d4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-files {
  font-size: 11px;
  color: #888;
  flex: 0 0 auto;
}

.dl-glyph {
  margin-left: auto;
  font-size: 12px;
  flex: 0 0 auto;
}

.glyph-downloading { color: #5a8af5; }
.glyph-paused { color: #f5a55a; }
.glyph-queued { color: #888; }
.glyph-completed { color: #4af54a; }
.glyph-failed { color: #f55a5a; }

.dl-track {
  height: 4px;
  background: #2a2a2a;
  border-radius: 2px;
  margin: 5px 0 4px;
  overflow: hidden;
}

.dl-fill {
  height: 100%;
  background: #5a8af5;
  border-radius: 2px;
  transition: width 0.25s;
}

.fill-done { background: #4af54a; }
.fill-failed { background: #f55a5a; }

.dl-meta {
  font-size: 11px;
  color: #888;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-actions {
  display: flex;
  gap: 4px;
  flex: 0 0 auto;
}

.dl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: #1e1e1e;
  border: 1px solid #333;
  border-radius: 5px;
  color: #aaa;
  cursor: pointer;
}

.dl-btn:hover { background: #2a2a2a; color: #fff; border-color: #5a8af5; }
</style>
