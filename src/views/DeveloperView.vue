<template>
  <div class="developer-view">
    <!-- Topbar -->
    <div class="topbar">
      <div class="dev-status">
        <span class="status-dot" :class="activeLoadedModel ? 'running' : 'stopped'"></span>
        <span class="status-text">{{ activeLoadedModel ? t('developer.running') : t('developer.stopped') }}</span>
      </div>
      <div style="flex:1"></div>
      <span style="color:#555; font-size:12px;">{{ t('developer.reachableAt') }}</span>
      <span style="color:#5a8af5; font-size:12px; margin: 0 8px;">{{ topbarUrl }}</span>
      <button class="btn-load" style="width:auto; padding: 5px 12px;" @click="showModal = true">+ {{ t('developer.loadModel') }}</button>
    </div>
    <!-- resto existente -->
    
    <LoadModelModal v-if="showModal" @close="showModal = false" />

    <!-- Loaded models -->
    <div v-if="modelLoading" class="loading-banner">
      {{ t('developer.loading') }}
    </div>
    <div class="dev-loaded-section">
      <div style="color:#666; font-size:11px; text-transform:uppercase; margin-bottom:8px;">{{ t('developer.loadedModels') }}</div>
      
      <div
        v-for="row in rows"
        :key="row.port"
        class="dev-model-row"
        :class="{ active: !row.loading && row.port === appConfig.chatPort, loading: row.loading }"
        @click="!row.loading && selectRow(row.port)"
      >
        <span v-if="row.loading" class="badge-loading">{{ t('developer.loading') }}</span>
        <span v-else class="badge-ready">{{ t('developer.ready') }}</span>

        <div v-if="!row.loading && genState[row.port]?.prefill !== null" class="prefill-progress">
          <div class="prefill-bar"><div class="prefill-fill" :style="{ width: genState[row.port]!.prefill + '%' }"></div></div>
          <span class="prefill-pct">{{ genState[row.port]!.prefill }}%</span>
        </div>
        <div v-if="!row.loading && genState[row.port]?.tokens !== null && genState[row.port]?.prefill === null" class="gen-tokens">
          <span style="color:#4af54a; font-size:11px;">{{ genState[row.port]!.tokens }} {{ t('developer.tokens') }}</span>
        </div>

        <span class="tag qwen" style="font-size:10px;">{{ row.model.arch }} {{ row.model.name }}</span>
        <span class="row-host" :class="{ copied: copiedPort === row.port }" :title="t('developer.copyUrl')" @click.stop="copyUrl(row.port, row.model)">
          {{ copiedPort === row.port ? t('developer.copied') : (hostByPath[row.model.path] ?? '127.0.0.1') + ':' + row.port }}
        </span>
        <div style="flex:1"></div>
        <span style="color:#555; font-size:11px;">{{ (row.model.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        <button v-if="!row.loading" class="btn-eject" @click.stop="ejectRow(row.port)">{{ t('developer.eject') }}</button>
      </div>

      <div v-if="rows.length === 0" style="color:#444; font-size:12px; padding:8px 0;">
        {{ t('developer.noModelLoaded') }}
      </div>
    </div>

    <!-- Logs -->
    <div class="dev-logs-section">
    <div class="dev-logs-header">
      <span style="color:#666; font-size:11px; text-transform:uppercase;">{{ t('developer.logs') }}</span>
      <button class="btn-clear-logs" @click="clearLogs">🗑 {{ t('developer.clearLogs') }}</button>
    </div>
      <div v-if="launchCmd" ref="launchEl" class="dev-launch" tabindex="-1" @keydown.ctrl.a.prevent="selectAllLaunch" @keydown.meta.a.prevent="selectAllLaunch">
        <div class="dev-launch-label">{{ t('developer.launch') }}</div>
        <div class="dev-launch-line">{{ launchCmd }}</div>
        <div v-if="launchSpec" class="dev-launch-line">{{ launchSpec }}</div>
      </div>
      <div class="dev-logs" ref="logsEl" tabindex="-1" @scroll.passive="onLogsScroll" @keydown.ctrl.a.prevent="selectAllLogs" @keydown.meta.a.prevent="selectAllLogs">
        <div v-for="(log, i) in displayLogs" :key="i" class="log-line">
          <span class="log-time">{{ log.time }}</span>
          <span :class="'log-level-' + log.level">{{ log.level.toUpperCase() }}</span>
          <span class="log-msg" v-html="highlightLog(log.msg, log.level)"></span>
        </div>
        <div v-if="displayLogs.length === 0" style="color:#444; padding:8px;">{{ t('developer.noLogs') }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed, onMounted, onUnmounted } from 'vue'
import { serverLogsByPort, launchCmdByPort, launchSpecByPort, activeLoadedModel, modelLoading, loadedModels, loadingModelFull, genState, removeLoaded, type ModelFile, type LogLine } from '../stores/selectedModel'
import { invoke } from '@tauri-apps/api/core'
import { appConfig, setChatPort, loadModelConfig } from '../stores/config'
import LoadModelModal from '../components/LoadModelModal.vue'
import { t } from '../i18n'

const logsEl = ref<HTMLElement>()
const launchEl = ref<HTMLElement>()
const showModal = ref(false)
const logs = computed<LogLine[]>(() => serverLogsByPort.value[appConfig.value.chatPort] ?? [])
const launchCmd = computed(() => launchCmdByPort.value[appConfig.value.chatPort] ?? '')
const launchSpec = computed(() => launchSpecByPort.value[appConfig.value.chatPort] ?? '')
const topbarUrl = computed(() =>
  activeLoadedModel.value
    ? `http://${hostByPath.value[activeLoadedModel.value.path] ?? '127.0.0.1'}:${appConfig.value.chatPort}`
    : `http://127.0.0.1:${appConfig.value.ports[0] ?? 8080}`
)

interface DevRow { port: number; model: ModelFile; loading: boolean }

const rows = computed<DevRow[]>(() => {
  const list: DevRow[] = Object.entries(loadedModels.value)
    .map(([p, m]) => ({ port: Number(p), model: m, loading: false }))
    .sort((a, b) => a.port - b.port)
  const lf = loadingModelFull.value
  if (lf && loadedModels.value[lf.port] === undefined) {
    list.push({ port: lf.port, model: lf.model, loading: true })
  }
  return list
})

// host por path (lazy cache: ModelConfig se lee una vez por modelo)
const hostByPath = ref<Record<string, string>>({})
watch(loadedModels, async (models) => {
  for (const m of Object.values(models)) {
    if (hostByPath.value[m.path] === undefined) {
      const cfg = await loadModelConfig(m.path)
      hostByPath.value[m.path] = cfg.host
    }
  }
}, { immediate: true })

const copiedPort = ref<number | null>(null)
let copyTimer: number | undefined
async function copyUrl(port: number, m: ModelFile) {
  const host = hostByPath.value[m.path] ?? '127.0.0.1'
  try {
    await navigator.clipboard.writeText(`http://${host}:${port}`)
    copiedPort.value = port
    if (copyTimer !== undefined) clearTimeout(copyTimer)
    copyTimer = window.setTimeout(() => { copiedPort.value = null }, 1500)
  } catch { /* clipboard bloqueado: sin feedback */ }
}

function selectRow(port: number) {
  setChatPort(port)
}

async function ejectRow(port: number) {
  await invoke('stop_model', { port })
  removeLoaded(port)
  const gs = genState.value
  if (gs[port]) {
    gs[port] = { prefill: null, tokens: null }
  }
  if (appConfig.value.chatPort === port) modelLoading.value = false
}

const AUTO_SCROLL_PAUSE_MS = 15000
const autoScrollPaused = ref(false)
const frozenLogs = ref<{time: string, level: string, msg: string}[]>([])
let resumeTimer: number | undefined

// Mientras el usuario está leyendo (alejado del fondo) la vista renderiza un
// snapshot fijo: el store sigue recopilando (y recortando a 1000) pero el DOM
// no se mueve, así que leer/copiar no se desplaza. Al reanudar vuelve al array vivo.
const displayLogs = computed(() =>
  autoScrollPaused.value ? frozenLogs.value : logs.value
)

function scrollLogsToBottom() {
  const el = logsEl.value
  if (el) el.scrollTop = el.scrollHeight
}

function isNearBottom(el: HTMLElement, threshold = 48): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight < threshold
}

function resumeAutoScroll() {
  if (resumeTimer !== undefined) {
    clearTimeout(resumeTimer)
    resumeTimer = undefined
  }
  frozenLogs.value = []
  autoScrollPaused.value = false
  scrollLogsToBottom()
}

// Cambiar de chatPort cambia el buffer de logs visible: reanudar el auto-scroll
watch(() => appConfig.value.chatPort, resumeAutoScroll)

function onLogsScroll() {
  const el = logsEl.value
  if (!el) return
  if (isNearBottom(el)) {
    resumeAutoScroll()
    return
  }
  if (!autoScrollPaused.value) {
    frozenLogs.value = [...logs.value]
    autoScrollPaused.value = true
  }
  if (resumeTimer !== undefined) clearTimeout(resumeTimer)
  resumeTimer = window.setTimeout(() => {
    resumeTimer = undefined
    resumeAutoScroll()
  }, AUTO_SCROLL_PAUSE_MS)
}

watch(displayLogs, () => {
  if (autoScrollPaused.value) return
  nextTick(scrollLogsToBottom)
}, { deep: true })

onMounted(() => {
  nextTick(() => {
    if (logsEl.value && !isNearBottom(logsEl.value)) scrollLogsToBottom()
  })
})

onUnmounted(() => {
  if (resumeTimer !== undefined) {
    clearTimeout(resumeTimer)
    resumeTimer = undefined
  }
})

function escHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function highlightLog(msg: string, level: string): string {
  if (level === 'error') {
    return `<span style="color:#f55a5a">${escHtml(msg)}</span>`
  }
  if (level === 'warn') {
    return `<span style="color:#f5c55a">${escHtml(msg)}</span>`
  }

  let result = escHtml(msg)

  result = result.replace(
    /(\d+\.?\d*)\s*(tokens per second)/g,
    '<span style="color:#4af54a;font-weight:600">$1</span> <span style="color:#2a8a2a">$2</span>'
  )

  result = result.replace(
    /(\d+\.?\d*)\s*(ms per token)/g,
    '<span style="color:#5a8af5">$1</span> <span style="color:#3a5aaa">$2</span>'
  )

  result = result.replace(
    /(prompt eval time|eval time|total time)\s*=\s*(\d+\.?\d*\s*ms)/g,
    '<span style="color:#5af5f5">$1</span> = <span style="color:#2aaaaa">$2</span>'
  )

  result = result.replace(
    /(draft acceptance)\s*=\s*(\d+\.?\d+)/g,
    '<span style="color:#f5a55a">$1</span> = <span style="color:#f5a55a;font-weight:600">$2</span>'
  )

  result = result.replace(
    /(\b(?:n_tokens|n_ctx|n_slots|n_ctx_slot)\s*=\s*)(\d+)/g,
    '<span style="color:#9a7af5">$1$2</span>'
  )

  result = result.replace(
    /(model loaded)/g,
    '<span style="color:#4af54a;font-weight:700">$1</span>'
  )

  result = result.replace(
    /(listening on\s+)(http[^\s]+)/g,
    '<span style="color:#aaa">$1</span><span style="color:#5a8af5;text-decoration:underline">$2</span>'
  )

  result = result.replace(
    /(loading model\s+')(.*?)(')/g,
    '<span style="color:#888">$1</span><span style="color:#ccc">$2</span><span style="color:#888">$3</span>'
  )

  result = result.replace(
    /(graphs reused\s*=\s*)(\d+)/g,
    '<span style="color:#666">$1$2</span>'
  )

  result = result.replace(
    /\btg\s*=\s*(\d+\.?\d*)\s*(t\/s)/g,
    'tg = <span style="color:#4af54a;font-weight:600">$1</span> <span style="color:#2a8a2a">$2</span>'
  )

  result = result.replace(
    /\btg_3s\s*=\s*(\d+\.?\d*)\s*(t\/s)/g,
    'tg_3s = <span style="color:#5af55a">$1</span> <span style="color:#2a8a2a">$2</span>'
  )

  result = result.replace(
    /\bn_gen\s*=\s*(\d+)/g,
    'n_gen = <span style="color:#9a7af5">$1</span>'
  )

  return result
}



function selectAllLogs() {
  const el = logsEl.value
  if (!el) return
  const range = document.createRange()
  range.selectNodeContents(el)
  const sel = window.getSelection()
  sel?.removeAllRanges()
  sel?.addRange(range)
}

function selectAllLaunch() {
  const el = launchEl.value
  if (!el) return
  const lines = el.querySelectorAll<HTMLElement>('.dev-launch-line')
  if (lines.length === 0) return
  const range = document.createRange()
  range.setStartBefore(lines[0])
  range.setEndAfter(lines[lines.length - 1])
  const sel = window.getSelection()
  sel?.removeAllRanges()
  sel?.addRange(range)
}

function clearLogs() {
  const port = appConfig.value.chatPort
  const logs = serverLogsByPort.value
  logs[port] = []
  serverLogsByPort.value = { ...logs }
  const gs = genState.value[port]
  if (gs) { gs.prefill = null; gs.tokens = null }
}
</script>
