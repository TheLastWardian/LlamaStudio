<template>
  <div class="settings-view">
    <div class="topbar">
      <span class="topbar-title">Settings</span>
    </div>

    <div class="settings-content">
      <div class="settings-section">
        <div class="section-title">Paths</div>

        <div class="settings-field">
          <label>Models Folder</label>
          <div class="path-row">
            <input type="text" v-model="config.modelsPath" class="field-input path-input" />
            <button class="btn-secondary" @click="browsePath('models')">Browse</button>
          </div>
        </div>

        <div class="settings-field">
          <label>llama-server.exe Path</label>
          <div class="path-row">
            <input type="text" v-model="config.llamaPath" class="field-input path-input" />
            <button class="btn-secondary" @click="browsePath('llama')">Browse</button>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">Server</div>
        <div class="settings-field">
          <label>Default Port</label>
          <input type="number" v-model="config.port" class="field-input" />
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">Behavior</div>
        <div class="settings-field">
          <label>Minimize to tray on close</label>
          <input type="checkbox" v-model="config.minimizeToTray" class="toggle" />
        </div>
      </div>

      <div class="settings-footer">
        <button class="btn-load" style="width:auto; padding: 6px 24px;" @click="save">Save</button>
        <span v-if="saved" style="color:#4af54a; font-size:12px;">Saved!</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { loadConfig, saveConfig, type AppConfig } from '../stores/config'

const config = ref<AppConfig>({
  modelsPath: '',
  llamaPath: '',
  port: 8080
})

const saved = ref(false)

onMounted(async () => {
  config.value = await loadConfig()
})

async function browsePath(type: 'models' | 'llama') {
  if (type === 'models') {
    const selected = await open({ directory: true })
    if (selected) config.value.modelsPath = selected as string
  } else {
    const selected = await open({ filters: [{ name: 'Executable', extensions: ['exe'] }] })
    if (selected) config.value.llamaPath = selected as string
  }
}

async function save() {
  await saveConfig(config.value)
  saved.value = true
  setTimeout(() => saved.value = false, 2000)
}
</script>
