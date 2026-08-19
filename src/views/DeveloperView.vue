<template>
  <div class="developer-view">
    <!-- Topbar -->
    <div class="topbar">
      <div class="dev-status">
        <span class="status-dot running"></span>
        <span class="status-text">Running</span>
      </div>
      <button class="btn-secondary">Server Settings</button>
      <button class="btn-secondary">mcp.json</button>
      <div style="flex:1"></div>
      <span style="color:#555; font-size:12px;">Reachable at</span>
      <span style="color:#5a8af5; font-size:12px; margin: 0 8px;">http://127.0.0.1:8080</span>
      <button class="btn-load" style="width:auto; padding: 5px 12px;">+ Load Model</button>
    </div>

    <!-- Loaded models -->
    <div v-if="modelLoading" class="loading-banner">
      ⏳ Loading model...
    </div>
    <div class="dev-loaded-section">
      <div style="color:#666; font-size:11px; text-transform:uppercase; margin-bottom:8px;">Loaded Models</div>
      <div class="dev-model-row">
        <span class="badge-ready">READY</span>
        <span class="tag qwen" style="font-size:10px;">llm qwen3.8-27b</span>
        <span class="tag" style="background:#1a3a1a; color:#5af55a; font-size:10px;">MTP</span>
        <div style="flex:1"></div>
        <span style="color:#555; font-size:11px;">Size 16.12 GB</span>
        <span style="color:#555; font-size:11px; margin: 0 12px;">Parallel 2</span>
        <button class="btn-eject">⏏ Eject</button>
      </div>
    </div>

    <!-- Logs -->
    <div class="dev-logs-section">
      <div class="dev-logs-header">
        <span style="color:#666; font-size:11px; text-transform:uppercase;">Developer Logs</span>
      </div>
      <div class="dev-logs" ref="logsEl">
        <div v-for="(log, i) in logs" :key="i" class="log-line">
          <span class="log-time">{{ log.time }}</span>
          <span :class="'log-level-' + log.level">{{ log.level.toUpperCase() }}</span>
          <span class="log-msg">{{ log.msg }}</span>
        </div>
        <div v-if="logs.length === 0" style="color:#444; padding:8px;">No logs yet.</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { modelLoading, serverLogs } from '../stores/selectedModel'

const logsEl = ref<HTMLElement>()
const logs = serverLogs

watch(logs, () => {
  nextTick(() => {
    if (logsEl.value) {
      logsEl.value.scrollTop = logsEl.value.scrollHeight
    }
  })
}, { deep: true })
</script>
