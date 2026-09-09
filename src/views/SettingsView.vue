<template>
  <div class="settings-view">
    <div class="topbar">
      <span class="topbar-title">{{ t('topbar.settings') }}</span>
      <button class="btn-load" :class="{ 'btn-blink': hasUnsaved }" style="width:auto; padding: 6px 24px;" @click="save">{{ t('settings.save') }}</button>
      <span v-if="saved" style="color:#4af54a; font-size:12px;">{{ t('settings.saved') }}</span>
      <span v-if="saveError" style="color:#f55a5a; font-size:12px;">{{ saveError }}</span>
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

      <div class="settings-grid">
      <div class="settings-col">
      <div class="settings-section">
        <div class="section-title">{{ t('settings.server') }}</div>
        <div class="settings-field">
          <label>{{ t('settings.modelsCount') }}</label>
          <select class="field-select" v-model.number="config.serverCount">
            <option v-for="n in 10" :key="n" :value="n">{{ n }}</option>
          </select>
        </div>
        <div class="ports-row">
          <div class="settings-field port-cell" v-for="(_p, i) in config.ports" :key="'port' + i">
            <label>{{ t('settings.modelPort', { n: i + 1 }) }}</label>
            <input type="number" v-model.number="config.ports[i]" min="1" max="65535" class="field-input" :class="portStateClass(i)" />
          </div>
        </div>
        <div class="settings-field">
          <label :title="t('settings.chatPortHint')">💬 {{ t('settings.chat') }} — {{ t('settings.chatPort') }}</label>
          <input type="number" v-model.number="config.chatPort" min="1" max="65535" class="field-input port-chat" />
        </div>
        <div class="settings-field">
          <label>{{ t('settings.onSlotsFull') }}</label>
          <select class="field-select" v-model="config.onSlotsFull">
            <option value="replace_first">{{ t('settings.onSlotsFullReplaceFirst') }}</option>
            <option value="ask">{{ t('settings.onSlotsFullAsk') }}</option>
          </select>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">{{ t('settings.downloads') }}</div>
        <div class="settings-field">
          <label :title="t('settings.parallelismTooltip')">{{ t('settings.parallelism') }}</label>
          <select class="field-select" v-model.number="config.downloads.parallelism">
            <option :value="1">1</option>
            <option :value="2">2</option>
            <option :value="3">3</option>
            <option :value="4">4</option>
          </select>
        </div>
        <div class="settings-field">
          <label :title="t('settings.chunksTooltip')">{{ t('settings.chunks') }}</label>
          <select class="field-select" v-model.number="config.downloads.chunks">
            <option :value="1">1</option>
            <option :value="2">2</option>
            <option :value="3">3</option>
            <option :value="4">4</option>
            <option :value="5">5</option>
            <option :value="6">6</option>
            <option :value="7">7</option>
            <option :value="8">8</option>
          </select>
        </div>
        <div class="settings-field">
          <label>{{ t('settings.autoRetry') }}</label>
          <input type="checkbox" v-model="config.downloads.autoRetry" class="toggle" />
        </div>
        <div class="settings-field">
          <label>{{ t('settings.maxRetries') }}</label>
          <input type="number" v-model.number="config.downloads.maxRetries" min="0" max="10" class="field-input" />
        </div>
        <div class="settings-field">
          <label>{{ t('settings.keepPartOnCancel') }}</label>
          <input type="checkbox" v-model="config.downloads.keepPartOnCancel" class="toggle" />
        </div>
      </div>
      </div>

      <div class="settings-col">
      <div class="settings-section">
        <div class="section-title">{{ t('settings.behavior') }}</div>
        <div class="settings-field">
          <label>{{ t('settings.minimizeToTray') }}</label>
          <input type="checkbox" v-model="config.minimizeToTray" class="toggle" />
        </div>
        <div class="settings-field">
          <label :title="t('settings.trashDeleteTooltip')">{{ t('settings.trashDelete') }}</label>
          <input type="checkbox" v-model="config.trashDelete" class="toggle" />
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
      </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { loadConfig, saveConfig, type AppConfig } from '../stores/config'
import { loadedModels } from '../stores/selectedModel'
import { t, setLang } from '../i18n'

const config = ref<AppConfig>({
  modelsPath: '',
  llamaPath: '',
  cudaGraphOpt: '',
  logVerbosity: 3,
  port: 8080,
  serverCount: 1,
  ports: [8080],
  chatPort: 8080,
  onSlotsFull: 'replace_first',
  minimizeToTray: false,
  trashDelete: false,
  language: 'en',
  blockedWords: [],
  downloads: { parallelism: 2, chunks: 4, autoRetry: true, maxRetries: 5, keepPartOnCancel: false },
})

const saved = ref(false)
const saveError = ref('')

// snapshot del último estado guardado: si el config actual difiere, hay cambios sin guardar
const savedSnapshot = ref('')
const hasUnsaved = computed(() =>
  savedSnapshot.value !== '' && JSON.stringify(config.value) !== savedSnapshot.value
)

// al cambiar la cantidad, redimensionar ports (completa con último+1 / trunca)
watch(() => config.value.serverCount, (n) => {
  const ports = config.value.ports
  while (ports.length < n) ports.push((ports[ports.length - 1] ?? 8080) + 1)
  if (ports.length > n) ports.length = n
  config.value.ports = [...ports]
})

function portStateClass(i: number): string {
  const p = config.value.ports[i]
  if (p === config.value.chatPort) return 'port-chat'
  return loadedModels.value[p] ? 'port-in-use' : ''
}

onMounted(async () => {
  config.value = await loadConfig()
  savedSnapshot.value = JSON.stringify(config.value)
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
  saveError.value = ''
  const ports = config.value.ports
  if (ports.some(p => p < 1 || p > 65535) || new Set(ports).size !== ports.length) {
    saveError.value = t('settings.invalidPorts')
    return
  }
  // modelos cargados cuyo puerto ya no está en la lista (reducción) → bloquear
  const blocked = Object.keys(loadedModels.value).map(Number).filter(p => !ports.includes(p))
  if (blocked.length > 0) {
    saveError.value = t('settings.reduceBlocked', { ports: blocked.join(', ') })
    return
  }
  await saveConfig(config.value)
  savedSnapshot.value = JSON.stringify(config.value)
  saved.value = true
  setTimeout(() => saved.value = false, 2000)
}
</script>
