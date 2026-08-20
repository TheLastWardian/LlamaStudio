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
          <input type="range" min="0" max="65" v-model.number="modelCfg.gpuOffload" class="slider" />
          <span class="slider-value">{{ modelCfg.gpuOffload }}</span>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">≋ {{ t('load.advanced') }}</div>
        <div class="field" :title="t('load.cpuThreadsTooltip')">
          <label>{{ t('load.cpuThreads') }}</label>
          <input type="number" v-model="modelCfg.cpuThreads" class="field-input" min="1" max="24" />
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
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field">
          <label>{{ t('load.vCacheQuant') }}</label>
          <select class="field-select" v-model="modelCfg.vCacheQuant">
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
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
        <div class="field" :title="t('load.cpuMoETooltip')">
          <label>{{ t('load.cpuMoE') }}</label>
          <input type="number" v-model="modelCfg.nCpuMoe" class="field-input" min="0" />
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
        <div class="field">
          <label>{{ t('load.reasoningEffort') }}</label>
          <select class="field-select" v-model="modelCfg.reasoningEffort">
            <option value="default">{{ t('load.default') }}</option>
            <option value="minimal">{{ t('load.minimal') }}</option>
            <option value="low">{{ t('load.low') }}</option>
            <option value="medium">{{ t('load.medium') }}</option>
            <option value="high">{{ t('load.high') }}</option>
            <option value="xhigh">{{ t('load.xhigh') }}</option>
            <option value="max">{{ t('load.max') }}</option>
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

      <div class="panel-footer">
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
    </div>

    <div v-if="activeTab === 'inference'">
      <div class="panel-section">
        <div class="section-title">{{ t('inference.systemPrompt') }}</div>
        <textarea 
          class="system-prompt" 
          :placeholder="t('inference.systemPromptPlaceholder')"
          v-model="modelCfg.systemPrompt"
        ></textarea>
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
import { ref, watch, computed } from 'vue'
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
    'flashAttention', 'specType', 'maxDraftTokens', 'kCacheQuant', 'vCacheQuant',
    'reasoningBudget', 'reasoningEffort', 'parallel', 'mlock', 'nCpuMoe',
    'host', 'noWarmup', 'sleepIdle', 'reasoningPreserve', 'fit', 'visionEnabled', 'mmprojPath', 'systemPrompt'
  ]
  
  return keys.some(k => String(modelCfg.value[k]) !== String(loadedModelConfig.value![k]))
})

const modelCfg = ref<ModelConfig>({
  contextLength: 100352,
  gpuOffload: 65,
  cpuThreads: 12,
  evalBatch: 2048,
  physicalBatch: 512,
  flashAttention: true,
  specType: 'MTP',
  maxDraftTokens: 2,
  draftProbability: 0.75,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
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
  nCpuMoe: 0,
  systemPrompt: '',
})

const maxContext = computed(() => selectedModel.value?.max_context || 262144)

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
    modelCfg.value = await loadModelConfig(model.path)
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
      port: config.port,
      host: modelCfg.value.host ?? '127.0.0.1',
      alias: modelCfg.value.alias ?? '',
      threadsHttp: Number(modelCfg.value.threadsHttp ?? 2),
      noWarmup: modelCfg.value.noWarmup ?? false,
      sleepIdle: Number(modelCfg.value.sleepIdle ?? -1),
      reasoningPreserve: modelCfg.value.reasoningPreserve ?? false,
      fit: modelCfg.value.fit ?? 'on',
      reasoningBudget: Number(modelCfg.value.reasoningBudget),
      reasoningEffort: modelCfg.value.reasoningEffort,
      parallel: Number(modelCfg.value.parallel ?? 1),
      mlock: modelCfg.value.mlock ?? false,
      mmap: modelCfg.value.mmap ?? false,
      kvUnified: modelCfg.value.kvUnified ?? false,
      nCpuMoe: Number(modelCfg.value.nCpuMoe ?? 0),
      visionEnabled: modelCfg.value.visionEnabled ?? false,
      mmprojPath: modelCfg.value.mmprojPath ?? '',
      systemPrompt: modelCfg.value.systemPrompt ?? '',
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
