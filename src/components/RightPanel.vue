<template>
  <div class="right-panel">
    <div class="panel-header">
      <span class="model-title">{{ selectedModel?.name ?? 'No model selected' }}</span>
      <div class="panel-tabs">
        <button class="tab" :class="{ active: activeTab === 'info' }" @click="activeTab = 'info'">Info</button>
        <button class="tab" :class="{ active: activeTab === 'load' }" @click="activeTab = 'load'">Load</button>
        <button class="tab" :class="{ active: activeTab === 'inference' }" @click="activeTab = 'inference'">Inference</button>
      </div>
    </div>

    <div v-if="activeTab === 'load'">
      <div class="panel-section">
        <div class="section-title">⚙ Context and Offload</div>
        <div class="field">
          <label>Context Length</label>
          <input type="number" v-model="modelCfg.contextLength" class="field-input" />
        </div>
        <div class="field">
          <label style="color:#555; font-size:11px;">Model supports up to {{ maxContext }} tokens</label>
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
          <label>GPU Offload</label>
          <input type="range" min="0" max="65" v-model.number="modelCfg.gpuOffload" class="slider" />
          <span class="slider-value">{{ modelCfg.gpuOffload }}</span>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">≋ Advanced</div>
        <div class="field" title="Hilos de CPU para generación de tokens. Máximo útil = threads de tu CPU (24 para Ryzen 9 5900x)">
          <label>CPU Thread Pool Size</label>
          <input type="number" v-model="modelCfg.cpuThreads" class="field-input" min="1" max="24" />
        </div>
        <div class="field">
          <label>Evaluation Batch Size</label>
          <input type="number" v-model.number="modelCfg.evalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>Physical Batch Size</label>
          <input type="number" v-model.number="modelCfg.physicalBatch" class="field-input" />
        </div>
        <div class="field">
          <label>Flash Attention</label>
          <input type="checkbox" v-model="modelCfg.flashAttention" class="toggle" />
        </div>
        <div class="field">
          <label>Speculative Decoding</label>
          <select class="field-select" v-model="modelCfg.specType">
            <option value="None">None</option>
            <option value="MTP">MTP</option>
            <option value="Draft">Draft Model</option>
          </select>
        </div>
        <template v-if="modelCfg.specType === 'Draft'">
          <div class="field">
            <label>Draft Model</label>
          </div>
          <select class="field-select" style="width:100%; margin-bottom:8px;" v-model="modelCfg.draftModelPath">
            <option value="">Select draft model...</option>
<option v-for="m in draftModels" :key="m.path" :value="m.path">
  {{ m.name }}
</option>
          </select>
        </template>
        <div class="field">
          <label>Max draft tokens</label>
          <input type="number" v-model.number="modelCfg.maxDraftTokens" class="field-input" />
        </div>
        <div class="field">
          <label>Draft probability</label>
          <input type="number" v-model.number="modelCfg.draftProbability" step="0.05" class="field-input" />
        </div>
        <div class="field">
          <label>K Cache Quantization</label>
          <select class="field-select" v-model="modelCfg.kCacheQuant">
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
        <div class="field">
          <label>V Cache Quantization</label>
          <select class="field-select" v-model="modelCfg.vCacheQuant">
            <option value="F16">F16</option>
            <option value="Q8_0">Q8_0</option>
            <option value="Q4_0">Q4_0</option>
          </select>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">🧠 Reasoning</div>
        <div class="field">
          <label>Reasoning Budget</label>
          <select class="field-select" v-model="modelCfg.reasoningBudget">
            <option value="-1">Unrestricted (default)</option>
            <option value="0">Off (disabled)</option>
            <option value="1024">1024 tokens</option>
            <option value="4096">4096 tokens</option>
            <option value="8192">8192 tokens</option>
            <option value="16384">16384 tokens</option>
          </select>
        </div>
        <div class="field">
          <label>Reasoning Effort</label>
          <select class="field-select" v-model="modelCfg.reasoningEffort">
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
        <div class="section-title">🌐 Server</div>
        <div class="field">
          <label>Host</label>
          <select class="field-select" v-model="modelCfg.host">
            <option value="127.0.0.1">localhost (127.0.0.1)</option>
            <option value="0.0.0.0">All interfaces (0.0.0.0)</option>
          </select>
        </div>
        <div class="field" title="Nombre del modelo expuesto en la API. Útil para identificarlo desde clientes como Open WebUI.">
          <label>Alias</label>
          <input type="text" v-model="modelCfg.alias" class="field-input" placeholder="optional" />
        </div>
        <div class="field" title="Hilos para procesar requests HTTP. 2-4 es suficiente para uso personal.">
          <label>HTTP Threads</label>
          <input type="number" v-model="modelCfg.threadsHttp" class="field-input" min="1" max="8" />
        </div>
        <div class="field" title="Desactiva el calentamiento inicial al cargar. Carga más rápido pero el primer request puede tardar más.">
          <label>No Warmup</label>
          <input type="checkbox" v-model="modelCfg.noWarmup" class="toggle" />
        </div>
        <div class="field" title="Segundos de inactividad antes de liberar VRAM. -1 = nunca dormir.">
          <label>Sleep Idle (seconds)</label>
          <input type="number" v-model="modelCfg.sleepIdle" class="field-input" min="-1" />
        </div>
        <div class="field" title="Mantiene el historial completo de thinking en el contexto, no solo el del último mensaje.">
          <label>Reasoning Preserve</label>
          <input type="checkbox" v-model="modelCfg.reasoningPreserve" class="toggle" />
        </div>
        <div class="field" title="'On' ajusta parámetros automáticamente para que el modelo entre en VRAM. 'Off' = control manual total.">
          <label>Fit</label>
          <select class="field-select" v-model="modelCfg.fit">
            <option value="on">On</option>
            <option value="off">Off</option>
          </select>
        </div>
      </div>

      <div class="panel-footer">
        <div v-if="error" style="color:#f55a5a; font-size:11px; margin-bottom:8px;">{{ error }}</div>
        <button class="btn-load" @click="loadModel" :disabled="loading">
          {{ loading ? 'Loading...' : '⬆ Load Model' }}
        </button>
        <button class="btn-secondary" style="width:100%; margin-top:6px;" @click="stopModel">
          ⏹ Stop
        </button>
      </div>
    </div>

    <div v-if="activeTab === 'inference'">
      <!-- Preset -->
      <div class="panel-section">
        <div class="section-title">Preset</div>
        <div class="field">
          <select class="field-select" style="flex:1">
            <option>Select a Preset...</option>
          </select>
        </div>
        <div class="field">
          <button class="btn-secondary" style="flex:1">+ Save Preset As...</button>
        </div>
      </div>

      <!-- System Prompt -->
      <div class="panel-section">
        <div class="section-title">System Prompt</div>
        <textarea
          class="system-prompt"
          placeholder='Example, "Only answer in rhymes"'
          v-model="systemPrompt"
        ></textarea>
        <div style="text-align:right; color:#555; font-size:11px; margin-top:4px;">Token count: N/A</div>
      </div>

      <!-- Reasoning -->
      <div class="panel-section">
        <div class="section-title">Reasoning</div>
        <div class="field">
          <label>Enable Thinking</label>
          <input type="checkbox" checked class="toggle" v-model="enableThinking" />
        </div>
        <div class="field" v-if="enableThinking">
          <label>Reasoning Budget</label>
          <span style="color:#555; font-size:12px;">Unrestricted</span>
        </div>
      </div>

      <!-- Settings -->
      <div class="panel-section">
        <div class="section-title">Settings</div>
        <div class="field">
          <label>Temperature</label>
          <input type="number" value="1" step="0.05" class="field-input" />
        </div>
        <div class="field">
          <label>Limit Response Length</label>
          <input type="checkbox" class="toggle" />
        </div>
        <div class="field">
          <label>Context Overflow</label>
          <select class="field-select">
            <option selected>Truncate Middle</option>
            <option>Stop</option>
            <option>Roll</option>
          </select>
        </div>
        <div class="field">
          <label>Stop Strings</label>
          <input type="text" class="field-input" placeholder="Enter string..." style="width:120px" />
        </div>
      </div>

      <!-- Sampling -->
      <div class="panel-section">
        <div class="section-title">Sampling</div>
        <div class="field">
          <label>Top K Sampling</label>
          <input type="number" value="20" class="field-input" />
        </div>
        <div class="field">
          <label>Repeat Penalty</label>
          <input type="number" value="1" step="0.05" class="field-input" />
        </div>
        <div class="field">
          <label>Presence Penalty</label>
          <input type="checkbox" class="toggle" />
        </div>
        <div class="field">
          <label>Top P Sampling</label>
          <input type="number" value="0.95" step="0.05" class="field-input" />
        </div>
        <div class="field">
          <label>Min P Sampling</label>
          <input type="number" value="0.05" step="0.05" class="field-input" />
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'info'">
      <div class="panel-section" v-if="selectedModel">
        <div class="section-title">ⓘ Model Information</div>
        
        <div class="info-row">
          <span class="info-label">Model</span>
          <span class="info-value tag-pill">{{ selectedModel.publisher }}/{{ selectedModel.model_family }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">File</span>
          <span class="info-value tag-pill">{{ selectedModel.name }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Format</span>
          <span class="info-value tag-pill">GGUF</span>
        </div>
        <div class="info-row">
          <span class="info-label">Quantization</span>
          <span class="info-value tag-pill">{{ selectedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Arch</span>
          <div style="display:flex; gap:6px; flex-wrap:wrap;">
            <span class="info-value tag-pill">{{ selectedModel.arch || '?' }}</span>
            <span class="info-value tag-pill" v-if="selectedModel.name.toLowerCase().includes('mtp')">MTP</span>
          </div>
        </div>
        <div class="info-row">
          <span class="info-label">Params</span>
          <span class="info-value tag-pill">{{ selectedModel.params || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Max Context</span>
          <span class="info-value tag-pill">{{ selectedModel.max_context?.toLocaleString() || '?' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Size on disk</span>
          <span class="info-value tag-pill">{{ (selectedModel.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        </div>
      </div>

      <div class="panel-section" v-if="selectedModel">
        <div class="section-title">🔗 API Usage</div>
        <div class="info-label" style="margin-bottom:6px;">The local server is reachable at:</div>
        <div class="copy-row">
          <span class="tag-pill copy-pill">http://127.0.0.1:8080</span>
          <button class="btn-copy" @click="copy('http://127.0.0.1:8080')">⧉</button>
        </div>
      </div>

      <div v-if="!selectedModel" style="padding:16px; color:#555; font-size:12px;">
        No model selected.
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, allModels, modelLoading, loadedModel } from '../stores/selectedModel'
import { loadConfig, loadModelConfig, saveModelConfig, type ModelConfig } from '../stores/config'

const activeTab = ref('load')

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

// Inference settings
const systemPrompt = ref('')
const enableThinking = ref(true)

const loading = ref(false)
const error = ref('')

async function loadModel() {
  console.log('loadModel called', selectedModel.value)
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
