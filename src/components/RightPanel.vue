<template>
  <div class="right-panel">
    <div class="panel-header">
      <span class="model-title">{{ selectedModel?.name ?? t('models.noModelSelected') }}</span>
      <div class="panel-tabs">
        <button class="tab" :class="{ active: activeTab === 'info' }" @click="activeTab = 'info'">{{ t('info.modelInfo') }}</button>
        <button class="tab" :class="{ active: activeTab === 'load' }" @click="activeTab = 'load'">{{ t('load.title') }}</button>
        <button class="tab" :class="{ active: activeTab === 'inference' }" @click="activeTab = 'inference'">{{ t('inference.settings') }}</button>
      </div>
    </div>

    <div v-if="activeTab === 'load'">
      <div class="panel-actions">
        <div v-if="error" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">{{ error }}</div>
        <button 
          class="btn-load" 
          :class="{ 'btn-reload-changes': hasUnsavedChanges }"
          @click="loadModel" 
          :disabled="loading"
        >
          {{ loading ? t('load.loading') : hasUnsavedChanges ? t('load.reloadChanges') : ((currentView === 'developer' && loadedModel) ? t('load.reload') : t('load.loadModel')) }}
        </button>
        <button class="btn-secondary" style="width:100%; margin-top:6px;" @click="stopModel">
          {{ t('load.stop') }}
        </button>
      </div>
      <div class="panel-section">
        <div class="section-title">⚙ Context and Offload</div>
        <div class="field">
          <label>{{ t('load.contextLength') }}</label>
          <input type="number" v-model="modelCfg.contextLength" class="field-input" />
        </div>
        <div class="field">
          <label style="color:#555; font-size:11px;">{{ t('load.modelSupportsTokens', { max: maxContext }) }}</label>
        </div>
        <input 
          type="range" 
          min="512" 
          :max="maxContext" 
          step="512"
          v-model="modelCfg.contextLength" 
          class="slider full-width"
        />
        <div class="field">
          <label>{{ t('load.gpuOffload') }}</label>
          <input type="range" min="0" :max="selectedModel?.layer_count ?? 999" v-model.number="modelCfg.gpuOffload" class="slider" />
          <span class="slider-value">{{ modelCfg.gpuOffload }}</span>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">≋ {{ t('load.advanced') }}</div>
        <div class="field" :title="t('load.cpuThreadsTooltip')">
          <label>{{ t('load.cpuThreads') }}</label>
          <input type="number" v-model="modelCfg.cpuThreads" class="field-input" min="1" max="128" />
        </div>
        <div class="field">
          <label>{{ t('load.evalBatch') }}</label>
          <input type="number" v-model.number="modelCfg.evalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.physicalBatch') }}</label>
          <input type="number" v-model.number="modelCfg.physicalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.flashAttention') }}</label>
          <input type="checkbox" v-model="modelCfg.flashAttention" class="toggle" />
        </div>
        <div class="field">
          <label>{{ t('load.speculativeDecoding') }}</label>
          <select class="field-select" v-model="modelCfg.specType">
            <option value="None">{{ t('load.none') }}</option>
            <option value="MTP">{{ t('load.mtp') }}</option>
            <option value="Draft">{{ t('load.draftModel') }}</option>
          </select>
        </div>
        <template v-if="modelCfg.specType === 'Draft'">
          <div class="field">
            <label>{{ t('load.draftModel') }}</label>
          </div>
          <select class="field-select" style="width:100%; margin-bottom:8px;" v-model="modelCfg.draftModelPath">
            <option value="">{{ t('load.selectDraft') }}</option>
<option v-for="m in draftModels" :key="m.path" :value="m.path">
  {{ m.name }}
</option>
          </select>
        </template>
        <div class="field">
          <label>{{ t('load.maxDraftTokens') }}</label>
          <input type="number" v-model.number="modelCfg.maxDraftTokens" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.draftProbability') }}</label>
          <input type="number" v-model.number="modelCfg.draftProbability" step="0.05" class="field-input" />
        </div>
        <div class="field">
          <label>{{ t('load.kCacheQuant') }}</label>
          <select class="field-select" v-model="modelCfg.kCacheQuant">
            <option value="F32">F32</option>
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field">
          <label>{{ t('load.vCacheQuant') }}</label>
          <select class="field-select" v-model="modelCfg.vCacheQuant">
            <option value="F32">F32</option>
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field" :title="t('load.cacheReuseTooltip')">
          <label>{{ t('load.cacheReuse') }}</label>
          <input type="number" v-model="modelCfg.cacheReuse" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.ctxCheckpointsTooltip')">
          <label>{{ t('load.ctxCheckpoints') }}</label>
          <input type="number" v-model="modelCfg.ctxCheckpoints" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.checkpointMinStepTooltip')">
          <label>{{ t('load.checkpointMinStep') }}</label>
          <input type="number" v-model="modelCfg.checkpointMinStep" class="field-input" min="0" />
        </div>
        <div class="field" :title="t('load.parallelTooltip')">
          <label>{{ t('load.parallelSlots') }}</label>
          <input type="number" v-model="modelCfg.parallel" class="field-input" min="1" max="16" />
        </div>
        <div class="field" :title="t('load.mlockTooltip')">
          <label>{{ t('load.mlock') }}</label>
          <input type="checkbox" v-model="modelCfg.mlock" class="toggle" />
        </div>
        <div class="field" :title="t('load.mmapTooltip')">
          <label>{{ t('load.mmap') }}</label>
          <input type="checkbox" v-model="modelCfg.mmap" class="toggle" />
        </div>
        <div class="field" :title="t('load.kvUnifiedTooltip')">
          <label>{{ t('load.kvUnified') }}</label>
          <input type="checkbox" v-model="modelCfg.kvUnified" class="toggle" />
        </div>
        <div class="field" :title="t('load.kvOffloadTooltip')">
          <label>{{ t('load.kvOffload') }}</label>
          <input type="checkbox" v-model="modelCfg.kvOffload" class="toggle" />
        </div>
        <div class="field" :title="t('load.cacheRamTooltip')">
          <label>{{ t('load.cacheRam') }}</label>
          <input type="number" v-model="modelCfg.cacheRam" class="field-input" min="-1" />
        </div>
        <input
          type="range"
          min="0"
          :max="systemRamTotal"
          step="256"
          v-model="modelCfg.cacheRam"
          class="slider full-width"
        />
        <div style="color:#555; font-size:11px; margin-bottom:4px;">
          {{ systemRamTotal > 0 ? `System RAM: ${systemRamTotal} MiB / Free: ${systemRamAvailable} MiB` : '' }}
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
        <div class="field" :title="t('load.seedTooltip')">
          <label>{{ t('load.seed') }}</label>
          <input type="number" v-model="modelCfg.seed" class="field-input" />
        </div>
        <div class="field" v-if="selectedModel?.is_moe" :title="t('load.cpuMoETooltip')">
          <label>{{ t('load.cpuMoE') }}</label>
          <input type="number" v-model="modelCfg.nCpuMoe" class="field-input" min="0" />
        </div>
        <div class="field" v-if="selectedModel?.is_moe" :title="t('load.numExpertsTooltip')">
          <label>{{ t('load.numExperts') }}</label>
          <input type="number" v-model="modelCfg.expertsPerToken" class="field-input" min="0" :max="selectedModel?.expert_count || undefined" :placeholder="selectedModel?.expert_used_count > 0 ? String(selectedModel?.expert_used_count) : ''" />
        </div>
      </div>

      <template v-if="selectedModel?.mmproj_paths?.length > 0">
        <div class="panel-section">
          <div class="section-title">👁️ {{ t('load.vision') }}</div>
          <div class="field" :title="t('load.visionTooltip')">
            <label>{{ t('load.visionLabel') }}</label>
            <input type="checkbox" v-model="modelCfg.visionEnabled" class="toggle" />
          </div>
          <div class="field" v-if="modelCfg.visionEnabled">
            <label>{{ t('load.mmprojModel') }}</label>
            <select class="field-select" v-model="modelCfg.mmprojPath" style="max-width:160px; font-size:10px;">
              <option v-for="p in selectedModel.mmproj_paths" :key="p" :value="p">
                {{ p.split('\\').pop() }}
              </option>
            </select>
          </div>
        </div>
      </template>

      <div class="panel-section">
        <div class="section-title">🧠 {{ t('load.reasoning') }}</div>
        <div class="field" v-if="selectedModel?.supports_thinking" :title="t('load.thinkingModeTooltip')">
          <label>{{ t('load.thinkingMode') }}</label>
          <select class="field-select" v-model="modelCfg.reasoning">
            <option value="auto">{{ t('load.auto') }}</option>
            <option value="on">{{ t('load.on') }}</option>
            <option value="off">{{ t('load.off') }}</option>
          </select>
        </div>
        <div class="field">
          <label>{{ t('load.reasoningBudget') }}</label>
          <select class="field-select" v-model="modelCfg.reasoningBudget">
            <option value="-1">{{ t('load.unrestricted') }}</option>
            <option value="0">{{ t('load.off') }}</option>
            <option value="1024">1024 tokens</option>
            <option value="4096">4096 tokens</option>
            <option value="8192">8192 tokens</option>
            <option value="16384">16384 tokens</option>
          </select>
        </div>
        <div class="field" v-if="selectedModel?.supports_effort">
          <label>{{ t('load.reasoningEffort') }}</label>
          <select class="field-select" v-model="modelCfg.reasoningEffort">
            <option v-for="lvl in effortOptions" :key="lvl" :value="lvl">{{ effortLabel(lvl) }}</option>
          </select>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">🌐 {{ t('load.server') }}</div>
        <div class="field">
          <label>{{ t('load.host') }}</label>
          <select class="field-select" v-model="modelCfg.host">
            <option value="127.0.0.1">{{ t('load.localhost') }}</option>
            <option value="0.0.0.0">{{ t('load.allInterfaces') }}</option>
          </select>
        </div>
        <div class="field" :title="t('load.aliasTooltip')">
          <label>{{ t('load.alias') }}</label>
          <input type="text" v-model="modelCfg.alias" class="field-input" :placeholder="t('load.optional')" />
        </div>
        <div class="field" :title="t('load.httpThreadsTooltip')">
          <label>{{ t('load.httpThreads') }}</label>
          <input type="number" v-model="modelCfg.threadsHttp" class="field-input" min="1" max="8" />
        </div>
        <div class="field" :title="t('load.noWarmupTooltip')">
          <label>{{ t('load.noWarmup') }}</label>
          <input type="checkbox" v-model="modelCfg.noWarmup" class="toggle" />
        </div>
        <div class="field" :title="t('load.sleepIdleTooltip')">
          <label>{{ t('load.sleepIdle') }}</label>
          <input type="number" v-model="modelCfg.sleepIdle" class="field-input" min="-1" />
        </div>
        <div class="field" :title="t('load.reasoningPreserveTooltip')">
          <label>{{ t('load.reasoningPreserve') }}</label>
          <input type="checkbox" v-model="modelCfg.reasoningPreserve" class="toggle" />
        </div>
        <div class="field" :title="t('load.fitTooltip')">
          <label>{{ t('load.fit') }}</label>
          <select class="field-select" v-model="modelCfg.fit">
            <option value="on">{{ t('load.fitOn') }}</option>
            <option value="off">{{ t('load.fitOff') }}</option>
          </select>
        </div>
      </div>

    </div>

    <div v-if="activeTab === 'inference'">
      <div class="panel-section">
        <div class="field">
          <label>{{ t('inference.temperature') }}</label>
          <input type="number" v-model="modelCfg.temp" class="field-input" step="0.05" min="0" max="2" />
        </div>
        <div class="field">
          <label>{{ t('inference.topP') }}</label>
          <input type="number" v-model="modelCfg.topP" class="field-input" step="0.01" min="0" max="1" />
        </div>
        <div class="field">
          <label>{{ t('inference.topK') }}</label>
          <input type="number" v-model="modelCfg.topK" class="field-input" step="1" min="0" />
        </div>
        <div class="field">
          <label>{{ t('inference.minP') }}</label>
          <input type="number" v-model="modelCfg.minP" class="field-input" step="0.01" min="0" max="1" />
        </div>
        <div class="field">
          <label>{{ t('inference.repeatPenalty') }}</label>
          <input type="number" v-model="modelCfg.repeatPenalty" class="field-input" step="0.01" min="0" />
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'info'">
      <div class="panel-section" v-if="selectedModel">
        <div class="section-title">ⓘ {{ t('info.modelInfo') }}</div>
        
        <div class="info-row">
          <span class="info-label">{{ t('info.model') }}</span>
          <span class="info-value tag-pill">{{ selectedModel.publisher }}/{{ selectedModel.model_family }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.file') }}</span>
          <span class="info-value tag-pill">{{ selectedModel.name }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.format') }}</span>
          <span class="info-value tag-pill">GGUF</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.quantization') }}</span>
          <span class="info-value tag-pill">{{ selectedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.arch') }}</span>
          <div style="display:flex; gap:6px; flex-wrap:wrap;">
            <span class="info-value tag-pill">{{ selectedModel.arch || '?' }}</span>
            <span class="info-value tag-pill" v-if="selectedModel.name.toLowerCase().includes('mtp')">MTP</span>
          </div>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.params') }}</span>
          <span class="info-value tag-pill">{{ selectedModel.params || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.maxContext') }}</span>
          <span class="info-value tag-pill">{{ selectedModel.max_context?.toLocaleString() || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">{{ t('info.sizeOnDisk') }}</span>
          <span class="info-value tag-pill">{{ (selectedModel.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        </div>
      </div>

      <div class="panel-section" v-if="selectedModel">
        <div class="section-title">🔗 {{ t('info.apiUsage') }}</div>
        <div class="info-label" style="margin-bottom:6px;">{{ t('info.serverReachable') }}</div>
        <div class="copy-row">
          <span class="tag-pill copy-pill">http://127.0.0.1:8080</span>
          <button class="btn-copy" @click="copy('http://127.0.0.1:8080')">⧉</button>
        </div>
      </div>

      <div v-if="!selectedModel" style="padding:16px; color:#555; font-size:12px;">
        {{ t('models.noModelSelected') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, allModels, modelLoading, loadedModel, loadingModel, loadedModelConfig } from '../stores/selectedModel'
import { loadConfig, loadModelConfig, saveModelConfig, type ModelConfig } from '../stores/config'
import { t } from '../i18n'

defineProps<{ currentView?: string }>()

const activeTab = ref('load')

const hasUnsavedChanges = computed(() => {
  if (!loadedModelConfig.value || !loadedModel.value) return false
  if (selectedModel.value?.path !== loadedModel.value.path) return false
  
  const keys: (keyof typeof modelCfg.value)[] = [
    'contextLength', 'gpuOffload', 'cpuThreads', 'evalBatch', 'physicalBatch',
    'flashAttention', 'specType', 'maxDraftTokens', 'kCacheQuant', 'vCacheQuant', 'cacheReuse',
    'ctxCheckpoints', 'checkpointMinStep',
    'reasoning', 'reasoningBudget', 'reasoningEffort', 'parallel', 'mlock', 'nCpuMoe', 'expertsPerToken',
    'mmap', 'kvUnified', 'seed', 'draftModelPath', 'threadsHttp', 'alias',
    'host', 'noWarmup', 'sleepIdle', 'reasoningPreserve', 'fit', 'visionEnabled', 'mmprojPath',
    'kvOffload', 'cacheRam', 'temp', 'topP', 'topK', 'minP', 'repeatPenalty'
  ]
  
  return keys.some(k => String(modelCfg.value[k]) !== String(loadedModelConfig.value![k]))
})

const effortOptions = computed(() => {
  const levels = selectedModel.value?.supported_effort_levels
  if (levels && levels.length > 0) return ['default', ...levels]
  return ['default', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']
})

const effortLabel = (v: string) => {
  const k = t('load.' + v)
  return k.startsWith('load.') ? v : k
}

const modelCfg = ref<ModelConfig>({
  contextLength: 4096,
  gpuOffload: 0,
  cpuThreads: 0,
  evalBatch: 2048,
  physicalBatch: 512,
  flashAttention: true,
  specType: 'MTP',
  maxDraftTokens: 2,
  draftProbability: 0.75,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
  cacheReuse: 0,
  ctxCheckpoints: 32,
  checkpointMinStep: 8192,
  reasoning: 'auto',
  reasoningBudget: '-1',
  reasoningEffort: 'default',
  draftModelPath: '',
  host: '127.0.0.1',
  alias: '',
  threadsHttp: 2,
  noWarmup: false,
  sleepIdle: -1,
  reasoningPreserve: false,
  fit: 'on',
  parallel: 1,
  mlock: false,
  mmap: false,
  kvUnified: false,
  kvOffload: false,
  cacheRam: 0,
  nCpuMoe: 0,
  expertsPerToken: 0,
  seed: -1,
  temp: 0.8,
  topP: 0.95,
  topK: 40,
  minP: 0.05,
  repeatPenalty: 1.0,
})

const maxContext = computed(() => selectedModel.value?.max_context || 262144)

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
  if (!systemRamAvailable.value || modelCfg.value.cacheRam <= 0) return 0
  return Math.round((modelCfg.value.cacheRam / systemRamAvailable.value) * 100)
})

const cacheRamWarning = computed(() => {
  const v = modelCfg.value.cacheRam
  if (v === -1) return 'unlimited'
  if (v <= 0 || !systemRamAvailable.value) return null
  if (v > systemRamAvailable.value) return 'critical'
  if (v > systemRamAvailable.value * 0.8) return 'warning'
  return null
})

const draftModels = computed(() => {
  if (!selectedModel.value) return []
  return allModels.value.filter(m => 
    m.model_family === selectedModel.value!.model_family &&
    m.path !== selectedModel.value!.path
  )
})

// Cargar config cuando cambia el modelo seleccionado
watch(selectedModel, async (model) => {
  if (model) {
    const cfg = await loadModelConfig(model.path)
    if (cfg.reasoningEffort !== 'default' && model.supported_effort_levels?.length && !model.supported_effort_levels.includes(cfg.reasoningEffort)) {
      cfg.reasoningEffort = 'default'
    }
    if (!cfg.cpuThreads || cfg.cpuThreads === 0) {
      cfg.cpuThreads = await invoke<number>('get_cpu_threads')
    }
    const maxLayers = model.layer_count ?? 999
    if (!cfg.gpuOffload || cfg.gpuOffload > maxLayers) {
      cfg.gpuOffload = maxLayers
    }
    modelCfg.value = cfg
  }
}, { immediate: true })

// Guardar config cuando cambia cualquier valor
watch(modelCfg, async (cfg) => {
  if (selectedModel.value) {
    await saveModelConfig(selectedModel.value.path, cfg)
  }
}, { deep: true })

const loading = ref(false)
const error = ref('')

async function loadModel() {
  console.log('loadModel called', selectedModel.value)
  loadingModel.value = selectedModel.value
  modelLoading.value = true
  loading.value = true
  error.value = ''

  try {
    const config = await loadConfig()
    console.log('config:', config)
    console.log('gpuLayers:', Number(modelCfg.value.gpuOffload))
    const result = await invoke<string>('load_model', {
      llamaPath: config.llamaPath,
      modelPath: selectedModel.value.path,
      gpuLayers: Number(modelCfg.value.gpuOffload),
      contextLength: Number(modelCfg.value.contextLength),
      cpuThreads: Number(modelCfg.value.cpuThreads),
      evalBatch: Number(modelCfg.value.evalBatch),
      physicalBatch: Number(modelCfg.value.physicalBatch),
      flashAttention: modelCfg.value.flashAttention,
      specType: modelCfg.value.specType,
      draftModelPath: modelCfg.value.draftModelPath,
      maxDraftTokens: Number(modelCfg.value.maxDraftTokens),
      draftProbability: modelCfg.value.draftProbability,
      kCacheQuant: modelCfg.value.kCacheQuant,
      vCacheQuant: modelCfg.value.vCacheQuant,
      cacheReuse: Number(modelCfg.value.cacheReuse ?? 0),
      ctxCheckpoints: Number(modelCfg.value.ctxCheckpoints ?? 32),
      checkpointMinStep: Number(modelCfg.value.checkpointMinStep ?? 8192),
      port: config.port,
      host: modelCfg.value.host ?? '127.0.0.1',
      alias: modelCfg.value.alias ?? '',
      threadsHttp: Number(modelCfg.value.threadsHttp ?? 2),
      noWarmup: modelCfg.value.noWarmup ?? false,
      sleepIdle: Number(modelCfg.value.sleepIdle ?? -1),
      reasoningPreserve: modelCfg.value.reasoningPreserve ?? false,
      fit: modelCfg.value.fit ?? 'on',
      reasoning: modelCfg.value.reasoning ?? 'auto',
      reasoningBudget: Number(modelCfg.value.reasoningBudget),
      reasoningEffort: modelCfg.value.reasoningEffort,
      parallel: Number(modelCfg.value.parallel ?? 1),
      mlock: modelCfg.value.mlock ?? false,
      mmap: modelCfg.value.mmap ?? false,
      kvUnified: modelCfg.value.kvUnified ?? false,
      kvOffload: modelCfg.value.kvOffload ?? false,
      cacheRam: Number(modelCfg.value.cacheRam ?? 0),
      nCpuMoe: Number(modelCfg.value.nCpuMoe ?? 0),
      expertsPerToken: Number(modelCfg.value.expertsPerToken ?? 0),
      visionEnabled: modelCfg.value.visionEnabled ?? false,
      mmprojPath: modelCfg.value.mmprojPath ?? '',
      seed: Number(modelCfg.value.seed ?? -1),
      temp: Number(modelCfg.value.temp ?? 0.8),
      topP: Number(modelCfg.value.topP ?? 0.95),
      topK: Number(modelCfg.value.topK ?? 40),
      minP: Number(modelCfg.value.minP ?? 0.05),
      repeatPenalty: Number(modelCfg.value.repeatPenalty ?? 1.0),
    })
    console.log('invoke result:', result)
    loadedModel.value = selectedModel.value
  } catch (e) {
    console.error('invoke error:', e)
    error.value = String(e)
    modelLoading.value = false
  } finally {
    loading.value = false
  }
}

async function stopModel() {
  await invoke('stop_model')
  modelLoading.value = false
  loadedModel.value = null
}

function copy(text: string) {
  navigator.clipboard.writeText(text)
}
</script>
