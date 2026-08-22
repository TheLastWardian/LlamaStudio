<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal" :class="{ 'modal-wide': view === 'config' }">
      
      <!-- Vista: Lista -->
      <template v-if="view === 'list'">
        <div class="modal-header">
          <input class="search-box" v-model="search" :placeholder="t('modal.filter')" autofocus />
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <!-- Modelo cargado actualmente -->
        <div v-if="loadedModel" class="modal-section-title">{{ t('modal.currentlyLoaded') }} (1)</div>
        <div v-if="loadedModel" class="modal-model-row loaded">
          <span class="pin-indicator" v-if="modelMeta[loadedModel.path]?.pinned">📌</span>
          <span class="modal-model-name">{{ modelDisplayNames[loadedModel.path] || loadedModel.name }}</span>
          <span class="tag quant">{{ loadedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
          <span class="tag" style="background:#1a2a1a; color:#4af54a;">GGUF</span>
          <div style="flex:1"></div>
          <button class="btn-eject" @click.stop="eject">⏏ {{ t('modal.eject') }}</button>
        </div>

        <!-- Lista agrupada -->
        <div class="modal-model-list">
          <template v-for="section in groupedFilteredModels" :key="section.groupName">
            <div v-if="section.models.length > 0" class="modal-section-title">
              {{ section.groupName }}
            </div>
            <div
              v-for="model in section.models"
              :key="model.path"
              class="modal-model-row"
              :class="{ active: loadedModel?.path === model.path }"
              @click="onModelClick($event, model)"
            >
              <span class="pin-indicator" v-if="modelMeta[model.path]?.pinned">📌</span>
              <span class="modal-model-name">{{ modelDisplayNames[model.path] || model.name }}</span>
              <span class="tag quant">{{ model.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
              <span class="tag" style="background:#2a2a2a; color:#888;">GGUF</span>
              <div style="flex:1"></div>
              <span style="color:#555; font-size:11px;">{{ (model.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
            </div>
          </template>
        </div>
      </template>

      <!-- Vista: Config -->
      <template v-else-if="view === 'config' && configModel && tempCfg">
        <div class="modal-header">
          <button class="btn-secondary" @click="view = 'list'">{{ t('modal.back') }}</button>
          <span style="color:#fff; font-size:13px; font-weight:600; flex:1; text-align:center;">
            {{ modelDisplayNames[configModel.path] || configModel.name }}
          </span>
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <div class="modal-config-body">
          <div class="panel-section">
            <div class="section-title">⚙ {{ t('load.contextAndOffload') }}</div>
            <div class="field">
              <label>{{ t('load.contextLength') }}</label>
              <input type="number" v-model="tempCfg.contextLength" class="field-input" />
            </div>
            <input type="range" min="512" :max="configModel.max_context || 262144" step="512" v-model="tempCfg.contextLength" class="slider full-width" />
            <div class="field">
              <label>{{ t('load.gpuOffload') }}</label>
              <input type="range" min="0" :max="configModel?.layer_count ?? 999" v-model="tempCfg.gpuOffload" class="slider" />
              <span class="slider-value">{{ tempCfg.gpuOffload }}</span>
            </div>
          </div>

          <div class="panel-section">
            <div class="section-title">≋ {{ t('load.advanced') }}</div>
            <div class="field">
              <label>{{ t('load.cpuThreads') }}</label>
              <input type="number" v-model="tempCfg.cpuThreads" class="field-input" min="1" max="128" />
            </div>
            <div class="field">
              <label>{{ t('load.evalBatch') }}</label>
              <input type="number" v-model="tempCfg.evalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>{{ t('load.physicalBatch') }}</label>
              <input type="number" v-model="tempCfg.physicalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>{{ t('load.parallelSlots') }}</label>
              <input type="number" v-model="tempCfg.parallel" class="field-input" min="1" max="16" />
            </div>
            <div class="field">
              <label>{{ t('load.flashAttention') }}</label>
              <input type="checkbox" v-model="tempCfg.flashAttention" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.mlock') }}</label>
              <input type="checkbox" v-model="tempCfg.mlock" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.mmap') }}</label>
              <input type="checkbox" v-model="tempCfg.mmap" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.kvUnified') }}</label>
              <input type="checkbox" v-model="tempCfg.kvUnified" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.kvOffload') }}</label>
              <input type="checkbox" v-model="tempCfg.kvOffload" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.cacheRam') }}</label>
              <input type="number" v-model="tempCfg.cacheRam" class="field-input" min="-1" />
            </div>
            <div v-if="cacheRamWarning === 'unlimited'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamUnlimited') }}
            </div>
            <div v-else-if="cacheRamWarning === 'critical'" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamCritical', { pct: cacheRamPct }) }}
            </div>
            <div v-else-if="cacheRamWarning === 'warning'" style="color:#f5a55a; font-size:11px; margin-bottom:8px;">
              {{ t('load.cacheRamWarning', { pct: cacheRamPct }) }}
            </div>
            <div class="field" v-if="configModel?.is_moe" :title="t('load.numExpertsTooltip')">
              <label>{{ t('load.numExperts') }}</label>
              <input type="number" v-model="tempCfg.expertsPerToken" class="field-input" min="0" :max="configModel?.expert_count || undefined" :placeholder="configModel?.expert_used_count > 0 ? String(configModel?.expert_used_count) : ''" />
            </div>
            <div class="field">
              <label>{{ t('load.cpuMoE') }}</label>
              <input type="number" v-model="tempCfg.nCpuMoe" class="field-input" min="0" />
            </div>
            <div class="field">
              <label>{{ t('load.speculativeDecoding') }}</label>
              <select class="field-select" v-model="tempCfg.specType">
                <option value="None">None</option>
                <option value="MTP">MTP</option>
                <option value="Draft">Draft Model</option>
              </select>
            </div>
            <template v-if="tempCfg.specType === 'Draft'">
              <div class="field">
                <label>{{ t('load.draftModel') }}</label>
              </div>
              <select class="field-select" style="width:100%; margin-bottom:8px;" v-model="tempCfg.draftModelPath">
                <option value="">{{ t('load.selectDraft') }}</option>
                <option v-for="m in draftModels" :key="m.path" :value="m.path">
                  {{ m.name }}
                </option>
              </select>
            </template>
            <div class="field">
              <label>{{ t('load.maxDraftTokens') }}</label>
              <input type="number" v-model="tempCfg.maxDraftTokens" class="field-input" min="1" />
            </div>
            <div class="field">
              <label>{{ t('load.draftProbability') }}</label>
              <input type="number" v-model="tempCfg.draftProbability" step="0.05" class="field-input" />
            </div>
            <div class="field">
              <label>{{ t('load.kCacheQuant') }}</label>
              <select class="field-select" v-model="tempCfg.kCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.vCacheQuant') }}</label>
              <select class="field-select" v-model="tempCfg.vCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
            <div class="field" :title="t('load.cacheReuseTooltip')">
              <label>{{ t('load.cacheReuse') }}</label>
              <input type="number" v-model="tempCfg.cacheReuse" class="field-input" min="0" />
            </div>
          </div>

          <template v-if="configModel?.mmproj_paths?.length > 0">
            <div class="panel-section">
              <div class="section-title">👁️ {{ t('load.vision') }}</div>
              <div class="field">
                <label>{{ t('load.visionLabel') }}</label>
                <input type="checkbox" v-model="tempCfg.visionEnabled" class="toggle" />
              </div>
              <div class="field" v-if="tempCfg.visionEnabled">
                <label>{{ t('load.mmprojModel') }}</label>
                <select class="field-select" v-model="tempCfg.mmprojPath" style="max-width:160px; font-size:10px;">
                  <option v-for="p in configModel.mmproj_paths" :key="p" :value="p">
                    {{ p.split('\\').pop() }}
                  </option>
                </select>
              </div>
            </div>
          </template>

          <div class="panel-section">
            <div class="section-title">🧠 {{ t('load.reasoning') }}</div>
            <div class="field">
              <label>{{ t('load.reasoningBudget') }}</label>
              <select class="field-select" v-model="tempCfg.reasoningBudget">
                <option value="-1">Unrestricted (default)</option>
                <option value="0">Off (disabled)</option>
                <option value="1024">1024 tokens</option>
                <option value="4096">4096 tokens</option>
                <option value="8192">8192 tokens</option>
                <option value="16384">16384 tokens</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.reasoningEffort') }}</label>
              <select class="field-select" v-model="tempCfg.reasoningEffort">
                <option value="default">Default</option>
                <option value="minimal">Minimal</option>
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
                <option value="xhigh">XHigh</option>
                <option value="max">Max</option>
              </select>
            </div>
          </div>

          <div class="panel-section">
            <div class="section-title">🌐 {{ t('load.server') }}</div>
            <div class="field">
              <label>{{ t('load.host') }}</label>
              <select class="field-select" v-model="tempCfg.host">
                <option value="127.0.0.1">localhost (127.0.0.1)</option>
                <option value="0.0.0.0">All interfaces (0.0.0.0)</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('load.alias') }}</label>
              <input type="text" v-model="tempCfg.alias" class="field-input" placeholder="optional" />
            </div>
            <div class="field">
              <label>{{ t('load.httpThreads') }}</label>
              <input type="number" v-model="tempCfg.threadsHttp" class="field-input" min="1" max="8" />
            </div>
            <div class="field">
              <label>{{ t('load.noWarmup') }}</label>
              <input type="checkbox" v-model="tempCfg.noWarmup" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.sleepIdle') }}</label>
              <input type="number" v-model="tempCfg.sleepIdle" class="field-input" min="-1" />
            </div>
            <div class="field">
              <label>{{ t('load.reasoningPreserve') }}</label>
              <input type="checkbox" v-model="tempCfg.reasoningPreserve" class="toggle" />
            </div>
            <div class="field">
              <label>{{ t('load.fit') }}</label>
              <select class="field-select" v-model="tempCfg.fit">
                <option value="on">On</option>
                <option value="off">Off</option>
              </select>
            </div>
          </div>
        </div>

        <div class="modal-config-footer">
          <button class="btn-secondary" @click="view = 'list'">{{ t('modal.back') }}</button>
          <button class="btn-load" style="width:auto; padding: 6px 24px;" @click="loadWithConfig">
            {{ t('modal.load') }}
          </button>
        </div>
      </template>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { allModels, loadedModel, selectedModel, modelLoading, loadingModel, loadedModelConfig } from '../stores/selectedModel'
import { modelDisplayNames, modelMeta, groups } from '../stores/groups'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig, loadModelConfig, saveModelConfig, type ModelConfig } from '../stores/config'
import { t } from '../i18n'
import type { ModelFile } from '../stores/selectedModel'

const search = ref('')
const view = ref<'list' | 'config'>('list')
const configModel = ref<ModelFile | null>(null)
const tempCfg = ref<ModelConfig | null>(null)
const emit = defineEmits(['close'])

const systemRamTotal = ref(0)
const systemRamAvailable = ref(0)

async function refreshSystemRam() {
  const [total, available] = await invoke<[number, number]>('get_system_ram')
  systemRamTotal.value = total
  systemRamAvailable.value = available
}

onMounted(() => {
  refreshSystemRam()
  window.addEventListener('focus', refreshSystemRam)
})

onUnmounted(() => {
  window.removeEventListener('focus', refreshSystemRam)
})

const cacheRamPct = computed(() => {
  if (!systemRamAvailable.value || !tempCfg.value || tempCfg.value.cacheRam <= 0) return 0
  return Math.round((tempCfg.value.cacheRam / systemRamAvailable.value) * 100)
})

const cacheRamWarning = computed(() => {
  const v = tempCfg.value?.cacheRam ?? 0
  if (v === -1) return 'unlimited'
  if (v <= 0 || !systemRamAvailable.value) return null
  if (v > systemRamAvailable.value) return 'critical'
  if (v > systemRamAvailable.value * 0.8) return 'warning'
  return null
})

const groupedFilteredModels = computed(() => {
  const filtered = allModels.value.filter(m =>
    m.path !== loadedModel.value?.path &&
    (m.name.toLowerCase().includes(search.value.toLowerCase()) ||
     (modelDisplayNames[m.path] ?? '').toLowerCase().includes(search.value.toLowerCase()))
  )

  const sortModel = (a: ModelFile, b: ModelFile) => {
    const aPinned = modelMeta[a.path]?.pinned ?? false
    const bPinned = modelMeta[b.path]?.pinned ?? false
    if (aPinned !== bPinned) return aPinned ? -1 : 1
    return (modelMeta[a.path]?.order ?? 0) - (modelMeta[b.path]?.order ?? 0)
  }

  const result: { groupName: string, models: ModelFile[] }[] = []

  const sortedGroups = [...groups.value].sort((a, b) => a.order - b.order)
  for (const group of sortedGroups) {
    const groupModels = filtered
      .filter(m => modelMeta[m.path]?.groupId === group.id)
      .sort(sortModel)
    result.push({ groupName: `📁 ${group.name}`, models: groupModels })
  }

  const ungrouped = filtered
    .filter(m => !modelMeta[m.path]?.groupId || !groups.value.find(g => g.id === modelMeta[m.path]?.groupId))
    .sort(sortModel)
  
  if (ungrouped.length > 0) {
    result.push({ groupName: groups.value.length > 0 ? 'Ungrouped' : 'Your Models', models: ungrouped })
  }

  return result
})

const draftModels = computed(() => {
  if (!configModel.value) return []
  return allModels.value.filter(m =>
    m.model_family === configModel.value!.model_family &&
    m.path !== configModel.value!.path
  )
})

async function onModelClick(e: MouseEvent, model: ModelFile) {
  if (e.altKey) {
    configModel.value = model
    tempCfg.value = { ...await loadModelConfig(model.path) }
    view.value = 'config'
  } else {
    selectModel(model)
  }
}

async function invokeLoad(modelPath: string, cfg: ModelConfig) {
  const config = await loadConfig()
  const cpuThreads = cfg.cpuThreads ?? 0
  const resolvedThreads = cpuThreads > 0 ? cpuThreads : await invoke<number>('get_cpu_threads')
  const gpuLayers = cfg.gpuOffload ?? 0
  const resolvedGpu = gpuLayers > 0 ? gpuLayers : (configModel.value?.layer_count ?? 999)
  await invoke('load_model', {
    llamaPath: config.llamaPath,
    modelPath,
    gpuLayers: resolvedGpu,
    contextLength: Number(cfg.contextLength ?? 4096),
    cpuThreads: resolvedThreads,
    evalBatch: Number(cfg.evalBatch ?? 2048),
    physicalBatch: Number(cfg.physicalBatch ?? 512),
    flashAttention: cfg.flashAttention ?? true,
    specType: cfg.specType ?? 'None',
    draftModelPath: cfg.draftModelPath ?? '',
    maxDraftTokens: Number(cfg.maxDraftTokens ?? 2),
    draftProbability: Number(cfg.draftProbability ?? 0.75),
    kCacheQuant: cfg.kCacheQuant ?? 'Q8_0',
    vCacheQuant: cfg.vCacheQuant ?? 'Q8_0',
    cacheReuse: Number(cfg.cacheReuse ?? 0),
    port: Number(config.port ?? 8080),
    host: cfg.host ?? '127.0.0.1',
    alias: cfg.alias ?? '',
    threadsHttp: Number(cfg.threadsHttp ?? 2),
    noWarmup: cfg.noWarmup ?? false,
    sleepIdle: Number(cfg.sleepIdle ?? -1),
    reasoningPreserve: cfg.reasoningPreserve ?? false,
    fit: cfg.fit ?? 'on',
    reasoningBudget: Number(cfg.reasoningBudget ?? -1),
    reasoningEffort: cfg.reasoningEffort ?? 'default',
    parallel: Number(cfg.parallel ?? 1),
    mlock: cfg.mlock ?? false,
    nCpuMoe: Number(cfg.nCpuMoe ?? 0),
    expertsPerToken: Number(cfg.expertsPerToken ?? 0),
    visionEnabled: cfg.visionEnabled ?? false,
    mmprojPath: cfg.mmprojPath ?? '',
    mmap: cfg.mmap ?? false,
    kvUnified: cfg.kvUnified ?? false,
    kvOffload: cfg.kvOffload ?? false,
    cacheRam: Number(cfg.cacheRam ?? 0),
    seed: Number(cfg.seed ?? -1),
    temp: Number(cfg.temp ?? 0.8),
    topP: Number(cfg.topP ?? 0.95),
    topK: Number(cfg.topK ?? 40),
    minP: Number(cfg.minP ?? 0.05),
    repeatPenalty: Number(cfg.repeatPenalty ?? 1.0),
  })
}

async function selectModel(model: ModelFile) {
  selectedModel.value = model
  loadingModel.value = model
  emit('close')
  modelLoading.value = true
  const cfg = await loadModelConfig(model.path)
  try {
    await invokeLoad(model.path, cfg)
  } catch (e) {
    modelLoading.value = false
    console.error(e)
  }
}

async function loadWithConfig() {
  if (!configModel.value || !tempCfg.value) return

  await saveModelConfig(configModel.value.path, tempCfg.value)
  
  selectedModel.value = configModel.value
  loadingModel.value = configModel.value
  emit('close')
  modelLoading.value = true
  try {
    await invokeLoad(configModel.value.path, tempCfg.value)
  } catch (e) {
    modelLoading.value = false
  }
}

async function eject() {
  await invoke('stop_model')
  loadedModel.value = null
  modelLoading.value = false
  emit('close')
}
</script>
