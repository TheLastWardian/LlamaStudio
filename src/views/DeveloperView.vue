<template>
  <div class="developer-view">
    <!-- Topbar -->
    <div class="topbar">
      <div class="dev-status">
        <span class="status-dot" :class="loadedModel ? 'running' : 'stopped'"></span>
        <span class="status-text">{{ loadedModel ? t('developer.running') : t('developer.stopped') }}</span>
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
      
      <div v-if="loadedModel" class="dev-model-row">
        <span class="badge-ready">{{ t('developer.ready') }}</span>
        <span class="tag qwen" style="font-size:10px;">{{ loadedModel.arch }} {{ loadedModel.name }}</span>
        <div style="flex:1"></div>
        <span style="color:#555; font-size:11px;">{{ (loadedModel.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        <button class="btn-eject" @click="eject">{{ t('developer.eject') }}</button>
      </div>
      
      <div v-else style="color:#444; font-size:12px; padding:8px 0;">
        {{ t('developer.noModelLoaded') }}
      </div>
    </div>

    <!-- Logs -->
    <div class="dev-logs-section">
      <div class="dev-logs-header">
        <span style="color:#666; font-size:11px; text-transform:uppercase;">{{ t('developer.developerLogs') }}</span>
      </div>
      <div class="dev-logs" ref="logsEl">
        <div v-for="(log, i) in logs" :key="i" class="log-line">
          <span class="log-time">{{ log.time }}</span>
          <span :class="'log-level-' + log.level">{{ log.level.toUpperCase() }}</span>
          <span class="log-msg" v-html="highlightLog(log.msg, log.level)"></span>
        </div>
        <div v-if="logs.length === 0" style="color:#444; padding:8px;">{{ t('developer.noLogs') }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted } from 'vue'
import { serverLogs, loadedModel, modelLoading } from '../stores/selectedModel'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig } from '../stores/config'
import LoadModelModal from '../components/LoadModelModal.vue'
import { t } from '../i18n'

const logsEl = ref<HTMLElement>()
const logs = serverLogs
const showModal = ref(false)
const port = ref(8080)

onMounted(async () => {
  const config = await loadConfig()
  port.value = config.port
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

  return result
}

async function eject() {
  await invoke('stop_model')
  loadedModel.value = null
  modelLoading.value = false
}

watch(logs, () => {
  nextTick(() => {
    if (logsEl.value) {
      logsEl.value.scrollTop = logsEl.value.scrollHeight
    }
  })
}, { deep: true })
</script>
