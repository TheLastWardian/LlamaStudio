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
      <ChatView v-if="currentView === 'chat'" />
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
import { serverLogs, modelLoading, selectedModel, loadedModel, loadingModel, loadedModelConfig } from './stores/selectedModel'
import { loadConfig, loadModelConfig } from './stores/config'
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

onMounted(async () => {
  const config = await loadConfig()
  setLang(config.language)
  const win = getCurrentWindow()
  await loadGroups()
  await invoke('load_window_state').catch(() => {})
  await win.show()

  if (unlistenLogs) unlistenLogs()
  
  unlistenLogs = await listen<string>('llama-log', (event) => {
    const line = event.payload
    const clean = line.replace(/\x1B\[[0-9;]*m/g, '').replace(/INFO/g, '')
    const match = clean.match(/^(\S+)\s+([IWED])\s+(.+)$/)
    const levelMap: Record<string, string> = { I: 'info', W: 'warn', E: 'error', D: 'debug' }
    serverLogs.value.push(
      match
        ? { time: match[1], level: levelMap[match[2]] ?? 'info', msg: match[3] }
        : { time: '', level: 'info', msg: clean }
    )
    
    if (clean.includes('model loaded')) {
      modelLoading.value = false
      loadedModel.value = loadingModel.value ?? selectedModel.value
      loadingModel.value = null
      if (loadedModel.value) {
        loadModelConfig(loadedModel.value.path).then(cfg => {
          loadedModelConfig.value = { ...cfg }
        })
      }
    }
    if (clean.includes('loading model')) {
      modelLoading.value = true
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
