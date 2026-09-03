<template>
  <div class="settings-view">
    <div class="topbar">
      <span class="topbar-title">{{ t('topbar.settings') }}</span>
    </div>

    <div class="settings-content">
      <div class="settings-section">
        <div class="section-title">{{ t('settings.paths') }}</div>

        <div class="settings-field">
          <label>{{ t('settings.modelsFolder') }}</label>
          <div class="path-row">
            <input type="text" v-model="config.modelsPath" class="field-input path-input" />
            <button class="btn-secondary" @click="browsePath('models')">{{ t('settings.browse') }}</button>
          </div>
        </div>

        <div class="settings-field">
          <label>{{ t('settings.llamaServer') }}</label>
          <div class="path-row">
            <input type="text" v-model="config.llamaPath" class="field-input path-input" />
            <button class="btn-secondary" @click="browsePath('llama')">{{ t('settings.browse') }}</button>
          </div>
        </div>

        <div class="settings-field">
          <label>{{ t('settings.cudaGraphOpt') }}</label>
          <input type="text" v-model="config.cudaGraphOpt" class="field-input" placeholder="1" />
        </div>

        <div class="settings-field">
          <label :title="t('settings.logVerbosityTooltip')">{{ t('settings.logVerbosity') }}</label>
          <input type="number" v-model.number="config.logVerbosity" min="1" max="5" class="field-input" placeholder="3" />
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">{{ t('settings.server') }}</div>
        <div class="settings-field">
          <label>{{ t('settings.defaultPort') }}</label>
          <input type="number" v-model="config.port" class="field-input" />
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">{{ t('settings.behavior') }}</div>
        <div class="settings-field">
          <label>{{ t('settings.minimizeToTray') }}</label>
          <input type="checkbox" v-model="config.minimizeToTray" class="toggle" />
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">{{ t('settings.language') }}</div>
        <div class="settings-field">
          <select class="field-select" v-model="config.language" @change="onLanguageChange">
            <option value="en">English</option>
            <option value="es">Español</option>
          </select>
        </div>
      </div>

      <div class="settings-footer">
        <button class="btn-load" style="width:auto; padding: 6px 24px;" @click="save">{{ t('settings.save') }}</button>
        <span v-if="saved" style="color:#4af54a; font-size:12px;">{{ t('settings.saved') }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { loadConfig, saveConfig, type AppConfig } from '../stores/config'
import { t, setLang } from '../i18n'

const config = ref<AppConfig>({
  modelsPath: '',
  llamaPath: '',
  cudaGraphOpt: '',
  logVerbosity: 3,
  port: 8080,
  minimizeToTray: false,
  language: 'en',
})

const saved = ref(false)

onMounted(async () => {
  config.value = await loadConfig()
  setLang(config.value.language)
})

function onLanguageChange() {
  setLang(config.value.language)
}

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
