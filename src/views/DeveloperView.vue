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
      <span style="color:#5a8af5; font-size:12px; margin: 0 8px;">http://127.0.0.1:{{ port }}</span>
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
      
      <div v-if="activeLoadedModel" class="dev-model-row">
        <span class="badge-ready">{{ t('developer.ready') }}</span>
        <div v-if="gen.prefill !== null" class="prefill-progress">
          <div class="prefill-bar"><div class="prefill-fill" :style="{ width: gen.prefill + '%' }"></div></div>
          <span class="prefill-pct">{{ gen.prefill }}%</span>
        </div>
        <div v-if="gen.tokens !== null && gen.prefill === null" class="gen-tokens">
          <span style="color:#4af54a; font-size:11px;">{{ gen.tokens }} {{ t('developer.tokens') }}</span>
        </div>
        <span class="tag qwen" style="font-size:10px;">{{ activeLoadedModel.arch }} {{ activeLoadedModel.name }}</span>
        <div style="flex:1"></div>
        <span style="color:#555; font-size:11px;">{{ (activeLoadedModel.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        <button class="btn-eject" @click="eject">{{ t('developer.eject') }}</button>
      </div>
      
      <div v-else style="color:#444; font-size:12px; padding:8px 0;">
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
import { serverLogsByPort, launchCmdByPort, launchSpecByPort, activeLoadedModel, activeGen, modelLoading, removeLoaded, genState, type LogLine } from '../stores/selectedModel'
import { invoke } from '@tauri-apps/api/core'
import { appConfig } from '../stores/config'
import LoadModelModal from '../components/LoadModelModal.vue'
import { t } from '../i18n'

const logsEl = ref<HTMLElement>()
const launchEl = ref<HTMLElement>()
const showModal = ref(false)
const logs = computed<LogLine[]>(() => serverLogsByPort.value[appConfig.value.chatPort] ?? [])
const launchCmd = computed(() => launchCmdByPort.value[appConfig.value.chatPort] ?? '')
const launchSpec = computed(() => launchSpecByPort.value[appConfig.value.chatPort] ?? '')
const port = computed(() => appConfig.value.chatPort)
const gen = computed(() => activeGen())

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

async function eject() {
  const port = appConfig.value.chatPort
  await invoke('stop_model', { port })
  removeLoaded(port)
  modelLoading.value = false
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
