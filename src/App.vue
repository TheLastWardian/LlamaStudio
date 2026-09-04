<template>
  <div class="app">
    <Sidebar :currentView="currentView" :previousView="previousView" @navigate="currentView = $event" />
    <div class="main">
      <div v-if="modelLoading" class="loading-overlay">
        <span class="loading-text">⏳ Loading {{ selectedModel?.name }}...</span>
        <div class="loading-bar">
          <div class="loading-bar-fill"></div>
        </div>
      </div>
      <ModelsView v-if="currentView === 'models'" />
      <ChatView v-show="currentView === 'chat'" />
      <DeveloperView v-if="currentView === 'developer'" />
      <SettingsView v-if="currentView === 'settings'" />
    </div>
    <div v-if="(currentView !== 'developer' && currentView !== 'chat') || loadedModel" class="resize-handle" @mousedown="startResize"></div>
    <RightPanel v-if="(currentView !== 'developer' && currentView !== 'chat') || loadedModel" :style="{ width: rightPanelWidth + 'px' }" :currentView="currentView" />
    <div v-else-if="currentView === 'developer'" style="width: 0px"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { serverLogs, modelLoading, selectedModel, loadedModel, loadingModel, loadedModelConfig, loadedServerPort, prefillProgress, generationTokens, type ModelFile } from './stores/selectedModel'
import { appConfig, loadConfig, loadModelConfig } from './stores/config'
import { loadGroups } from './stores/groups'
import { setLang } from './i18n'
import Sidebar from './components/Sidebar.vue'
import ModelsView from './views/ModelsView.vue'
import ChatView from './views/ChatView.vue'
import DeveloperView from './views/DeveloperView.vue'
import SettingsView from './views/SettingsView.vue'
import RightPanel from './components/RightPanel.vue'

const currentView = ref('models')
const previousView = ref('models')
const rightPanelWidth = ref(300)

let unlistenLogs: (() => void) | null = null

watch(currentView, (newView, oldView) => {
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

async function restoreLoadedModel(port: number, modelsPath: string) {
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
      model = { name, publisher: '', model_family: '', size_bytes: 0, path: id, arch: '', params: '', max_context: 0, layer_count: 0, is_moe: false, expert_count: 0, expert_used_count: 0, mmproj_paths: [] }
    }

    selectedModel.value = model
    loadedModel.value = model
    loadedServerPort.value = port
    modelLoading.value = false
    loadingModel.value = null
    loadedModelConfig.value = { ...(await loadModelConfig(model.path)) }
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
  const win = getCurrentWindow()
  await loadGroups()
  await invoke('load_window_state').catch(() => {})
  await win.show()

  restoreLoadedModel(config.port, config.modelsPath)

  if (unlistenLogs) unlistenLogs()
  
  unlistenLogs = await listen<string>('llama-log', (event) => {
    const line = event.payload
    const clean = line.replace(/\x1B\[[0-9;]*m/g, '')
    const match = clean.match(/^(\S+)\s+([IWED])\s+(.+)$/)
    const levelMap: Record<string, string> = { I: 'info', W: 'warn', E: 'error', D: 'debug' }
    serverLogs.value.push(
      match
        ? { time: fmtLogTime(match[1]), level: levelMap[match[2]] ?? 'info', msg: match[3] }
        : { time: '', level: 'info', msg: clean }
    )
    if (serverLogs.value.length > 1000) serverLogs.value.splice(0, serverLogs.value.length - 1000)

    if (clean.includes('print_timing')) {
      if (clean.includes('prompt processing')) {
        const m = clean.match(/progress = ([\d.]+)/)
        if (m) prefillProgress.value = Math.round(parseFloat(m[1]) * 100)
      } else {
        prefillProgress.value = null
      }
      if (clean.includes('n_gen')) {
        const m = clean.match(/n_gen\s*=\s*(\d+)/)
        if (m) generationTokens.value = parseInt(m[1])
      }
    }
    if (clean.includes('slot release')) {
      prefillProgress.value = null
      generationTokens.value = null
    }

    if (clean.includes('model loaded')) {
      modelLoading.value = false
      prefillProgress.value = null
      generationTokens.value = null
      const target = loadingModel.value ?? selectedModel.value
      if (target) {
        loadedModel.value = target
        loadingModel.value = null
        loadModelConfig(target.path).then(cfg => {
          loadedModelConfig.value = { ...cfg }
        })
      } else {
        restoreLoadedModel(appConfig.value.port, config.modelsPath)
      }
    }
    if (clean.includes('loading model')) {
      modelLoading.value = true
    }
  })

  await listen('llama-exited', () => {
    if (!loadedModel.value && !modelLoading.value) return
    serverLogs.value.push({ time: '', level: 'error', msg: 'server process exited' })
    if (serverLogs.value.length > 1000) serverLogs.value.splice(0, serverLogs.value.length - 1000)
    loadedModel.value = null
    loadingModel.value = null
    loadedServerPort.value = null
    modelLoading.value = false
    prefillProgress.value = null
    generationTokens.value = null
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
