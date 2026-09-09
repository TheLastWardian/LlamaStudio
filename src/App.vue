<template>
  <div class="app">
    <Sidebar :currentView="currentView" :previousView="previousView" @navigate="currentView = $event" />
    <div class="main">
      <div v-if="modelLoading" class="loading-overlay">
        <span class="loading-text">{{ t('app.loading', { name: selectedModel?.name ?? '' }) }}</span>
        <div class="loading-bar">
          <div class="loading-bar-fill"></div>
        </div>
      </div>
      <ModelsView v-if="currentView === 'models'" />
      <ChatView v-show="currentView === 'chat'" />
      <DeveloperView v-if="currentView === 'developer'" />
      <DiscoverView v-if="currentView === 'discover'" />
      <SettingsView v-if="currentView === 'settings'" />
      <DownloadsBar />
    </div>
    <div v-if="currentView !== 'discover' && ((currentView !== 'developer' && currentView !== 'chat') || activeLoadedModel)" class="resize-handle" @mousedown="startResize"></div>
    <RightPanel v-if="currentView !== 'discover' && ((currentView !== 'developer' && currentView !== 'chat') || activeLoadedModel)" :style="{ width: rightPanelWidth + 'px' }" :currentView="currentView" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { serverLogsByPort, launchCmdByPort, launchSpecByPort, modelLoading, selectedModel, loadingModelFull, genState, setLoaded, removeLoaded, activeLoadedModel, type ModelFile } from './stores/selectedModel'
import { loadConfig, loadModelConfig } from './stores/config'
import { loadGroups } from './stores/groups'
import { loadColumnWidths } from './stores/columnWidths'
import { setLang, t } from './i18n'
import Sidebar from './components/Sidebar.vue'
import ModelsView from './views/ModelsView.vue'
import ChatView from './views/ChatView.vue'
import DeveloperView from './views/DeveloperView.vue'
import DiscoverView from './views/DiscoverView.vue'
import SettingsView from './views/SettingsView.vue'
import DownloadsBar from './components/DownloadsBar.vue'
import RightPanel from './components/RightPanel.vue'
import { init as initDownloads } from './stores/downloads'

const currentView = ref('models')
const previousView = ref('models')
const rightPanelWidth = ref(300)

let unlistenLogs: (() => void) | null = null

watch(currentView, (newView) => {
  if (newView !== 'settings') {
    previousView.value = newView
  }
})

function startResize(e: MouseEvent) {
  e.preventDefault()
  const startX = e.clientX
  const startWidth = rightPanelWidth.value

  function onMove(e: MouseEvent) {
    const diff = startX - e.clientX
    rightPanelWidth.value = Math.max(220, Math.min(600, startWidth + diff))
  }

  function onUp() {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }

  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

async function restorePort(port: number, modelsPath: string) {
  try {
    const ctrl = new AbortController()
    const timer = setTimeout(() => ctrl.abort(), 2000)
    const res = await fetch(`http://127.0.0.1:${port}/v1/models`, { signal: ctrl.signal })
    clearTimeout(timer)
    if (!res.ok) return
    const json: any = await res.json()
    const id: string | undefined = json?.data?.[0]?.id
    if (!id) return

    const models: ModelFile[] = await invoke('scan_models', { modelsPath })
    let model = models.find(m => m.path === id)
      ?? models.find(m => m.path.replace(/\\/g, '/') === id.replace(/\\/g, '/'))
      ?? null
    if (!model) {
      for (const m of models) {
        const cfg = await loadModelConfig(m.path)
        if (cfg.alias && cfg.alias === id) { model = m; break }
      }
    }
    if (!model) {
      const name = id.split(/[\\/]/).pop() || id
      model = { name, publisher: '', model_family: '', size_bytes: 0, path: id, arch: '', params: '', max_context: 0, layer_count: 0, embedding_length: 0, head_count: 0, head_count_kv: 0, key_length: 0, sliding_window: 0, sliding_window_pattern: '', key_length_swa: 0, head_count_kv_list: '', shared_kv_layers: 0, full_attention_interval: 0, ssm_state_size: 0, ssm_inner_size: 0, feed_forward_length: 0, expert_feed_forward_length: 0, is_moe: false, expert_count: 0, expert_used_count: 0, supports_thinking: false, supports_effort: false, supported_effort_levels: [], mmproj_paths: [], is_draft: false }
    }
    setLoaded(port, model)
  } catch {
    // server no corriendo o aún cargando — nada que restaurar
  }
}

function fmtLogTime(t: string): string {
  const parts = t.split('.')
  if (parts.length !== 4) return t
  const min = parseInt(parts[0], 10)
  const sec = parseInt(parts[1], 10)
  const ms = parseInt(parts[2], 10)
  if (isNaN(min) || isNaN(sec) || isNaN(ms)) return t
  const totalSec = min * 60 + sec
  return `${String(Math.floor(totalSec / 60)).padStart(2, '0')}:${String(totalSec % 60).padStart(2, '0')}.${String(ms).padStart(3, '0')}`
}

onMounted(async () => {
  const config = await loadConfig()
  setLang(config.language)
  // suscribe download-progress/download-state una sola vez + carga jobs persistidos
  initDownloads()
  const win = getCurrentWindow()
  await loadGroups()
  await loadColumnWidths()
  await invoke('load_window_state').catch(() => {})
  await win.show()

  for (const port of config.ports) restorePort(port, config.modelsPath)

  unlistenLogs = await listen<{ port: number; line: string }>('llama-log', (event) => {
    const port = event.payload.port
    const clean = event.payload.line.replace(/\x1B\[[0-9;]*m/g, '')
    const cmd = launchCmdByPort.value
    const spec = launchSpecByPort.value
    if (clean.startsWith('CMD:')) cmd[port] = clean
    else if (clean.startsWith('SPEC:')) spec[port] = clean
    const match = clean.match(/^(\S+)\s+([IWED])\s+(.+)$/)
    const levelMap: Record<string, string> = { I: 'info', W: 'warn', E: 'error', D: 'debug' }
    const logs = serverLogsByPort.value[port] ?? []
    logs.push(
      match
        ? { time: fmtLogTime(match[1]), level: levelMap[match[2]] ?? 'info', msg: match[3] }
        : { time: '', level: 'info', msg: clean }
    )
    if (logs.length > 1000) logs.splice(0, logs.length - 1000)
    serverLogsByPort.value = { ...serverLogsByPort.value, [port]: logs }

    const gs = genState.value[port]
    if (gs) {
      if (clean.includes('print_timing')) {
        if (clean.includes('prompt processing')) {
          const m = clean.match(/progress = ([\d.]+)/)
          if (m) gs.prefill = Math.round(parseFloat(m[1]) * 100)
        } else {
          gs.prefill = null
        }
        if (clean.includes('n_gen')) {
          const m = clean.match(/n_gen\s*=\s*(\d+)/)
          if (m) gs.tokens = parseInt(m[1])
        }
      }
      if (clean.includes('slot release')) {
        gs.prefill = null
        gs.tokens = null
      }
    }

    if (clean.includes('model loaded')) {
      modelLoading.value = false
      const pending = loadingModelFull.value
      if (pending && pending.port === port) {
        setLoaded(port, pending.model)
        loadingModelFull.value = null
      } else {
        // server que terminó de cargar sin carga pendiente (restore temprano: el
        // fetch a /v1/models falló porque el modelo aún cargaba; o crash y relaunch)
        // → reintentar el restore. Idempotente: si ya está cargado, setLoaded repite.
        restorePort(port, config.modelsPath)
      }
    }
    if (clean.includes('loading model')) {
      modelLoading.value = true
    }
  })

  await listen<{ port: number }>('llama-exited', (event) => {
    const port = event.payload.port
    const logs = serverLogsByPort.value[port] ?? []
    logs.push({ time: '', level: 'error', msg: 'server process exited' })
    if (logs.length > 1000) logs.splice(0, logs.length - 1000)
    serverLogsByPort.value = { ...serverLogsByPort.value, [port]: logs }
    removeLoaded(port)
    if (loadingModelFull.value?.port === port) {
      loadingModelFull.value = null
      modelLoading.value = false
    }
  })

  await win.listen('tauri://close-requested', async () => {
    const config = await loadConfig()
    if (config.minimizeToTray) {
      await win.hide()
    } else {
      await invoke('save_window_state').catch(() => {})
      await win.destroy()
    }
  })
})

onUnmounted(() => {
  if (unlistenLogs) unlistenLogs()
})
</script>
