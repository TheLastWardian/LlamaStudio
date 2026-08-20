<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal" :class="{ 'modal-wide': view === 'config' }">
      
      <!-- Vista: Lista -->
      <template v-if="view === 'list'">
        <div class="modal-header">
          <input class="search-box" v-model="search" placeholder="Type to filter models..." autofocus />
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <!-- Modelo cargado actualmente -->
        <div v-if="loadedModel" class="modal-section-title">Currently Loaded (1)</div>
        <div v-if="loadedModel" class="modal-model-row loaded">
          <span class="pin-indicator" v-if="modelMeta[loadedModel.path]?.pinned">📌</span>
          <span class="modal-model-name">{{ modelDisplayNames[loadedModel.path] || loadedModel.name }}</span>
          <span class="tag quant">{{ loadedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
          <span class="tag" style="background:#1a2a1a; color:#4af54a;">GGUF</span>
          <div style="flex:1"></div>
          <button class="btn-eject" @click.stop="eject">⏏ Eject</button>
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
          <button class="btn-secondary" @click="view = 'list'">← Back</button>
          <span style="color:#fff; font-size:13px; font-weight:600; flex:1; text-align:center;">
            {{ modelDisplayNames[configModel.path] || configModel.name }}
          </span>
          <button class="modal-close" @click="$emit('close')">✕</button>
        </div>

        <div class="modal-config-body">
          <!-- Context -->
          <div class="panel-section">
            <div class="section-title">⚙ Context and Offload</div>
            <div class="field">
              <label>Context Length</label>
              <input type="number" v-model="tempCfg.contextLength" class="field-input" />
            </div>
            <input type="range" min="512" :max="configModel.max_context || 262144" step="512" v-model="tempCfg.contextLength" class="slider full-width" />
            <div class="field">
              <label>GPU Offload</label>
              <input type="range" min="0" max="65" v-model="tempCfg.gpuOffload" class="slider" />
              <span class="slider-value">{{ tempCfg.gpuOffload }}</span>
            </div>
          </div>

          <!-- Advanced -->
          <div class="panel-section">
            <div class="section-title">≋ Advanced</div>
            <div class="field">
              <label>CPU Thread Pool Size</label>
              <input type="number" v-model="tempCfg.cpuThreads" class="field-input" min="1" max="24" />
            </div>
            <div class="field">
              <label>Evaluation Batch Size</label>
              <input type="number" v-model="tempCfg.evalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>Physical Batch Size</label>
              <input type="number" v-model="tempCfg.physicalBatch" class="field-input" />
            </div>
            <div class="field">
              <label>Parallel Slots</label>
              <input type="number" v-model="tempCfg.parallel" class="field-input" min="1" max="16" />
            </div>
            <div class="field">
              <label>Flash Attention</label>
              <input type="checkbox" v-model="tempCfg.flashAttention" class="toggle" />
            </div>
            <div class="field">
              <label>mlock</label>
              <input type="checkbox" v-model="tempCfg.mlock" class="toggle" />
            </div>
            <div class="field">
              <label>mmap</label>
              <input type="checkbox" v-model="tempCfg.mmap" class="toggle" />
            </div>
            <div class="field">
              <label>kv-unified</label>
              <input type="checkbox" v-model="tempCfg.kvUnified" class="toggle" />
            </div>
            <div class="field">
              <label>CPU MoE Layers</label>
              <input type="number" v-model="tempCfg.nCpuMoe" class="field-input" min="0" />
            </div>
            <div class="field">
              <label>Speculative Decoding</label>
              <select class="field-select" v-model="tempCfg.specType">
                <option value="None">None</option>
                <option value="MTP">MTP</option>
                <option value="Draft">Draft Model</option>
              </select>
            </div>
            <div class="field">
              <label>Max Draft Tokens</label>
              <input type="number" v-model="tempCfg.maxDraftTokens" class="field-input" min="1" />
            </div>
            <div class="field">
              <label>K Cache Quantization</label>
              <select class="field-select" v-model="tempCfg.kCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
            <div class="field">
              <label>V Cache Quantization</label>
              <select class="field-select" v-model="tempCfg.vCacheQuant">
                <option>F16</option>
                <option>Q8_0</option>
                <option>Q4_0</option>
              </select>
            </div>
          </div>

          <!-- Server -->
          <div class="panel-section">
            <div class="section-title">🌐 Server</div>
            <div class="field">
              <label>Host</label>
              <select class="field-select" v-model="tempCfg.host">
                <option value="127.0.0.1">localhost (127.0.0.1)</option>
                <option value="0.0.0.0">All interfaces (0.0.0.0)</option>
              </select>
            </div>
            <div class="field">
              <label>Alias</label>
              <input type="text" v-model="tempCfg.alias" class="field-input" placeholder="optional" />
            </div>
            <div class="field">
              <label>HTTP Threads</label>
              <input type="number" v-model="tempCfg.threadsHttp" class="field-input" min="1" max="8" />
            </div>
            <div class="field">
              <label>No Warmup</label>
              <input type="checkbox" v-model="tempCfg.noWarmup" class="toggle" />
            </div>
            <div class="field">
              <label>Sleep Idle (seconds)</label>
              <input type="number" v-model="tempCfg.sleepIdle" class="field-input" min="-1" />
            </div>
            <div class="field">
              <label>Reasoning Preserve</label>
              <input type="checkbox" v-model="tempCfg.reasoningPreserve" class="toggle" />
            </div>
            <div class="field">
              <label>Fit</label>
              <select class="field-select" v-model="tempCfg.fit">
                <option value="on">On</option>
                <option value="off">Off</option>
              </select>
            </div>
          </div>

          <!-- Reasoning -->
          <div class="panel-section">
            <div class="section-title">🧠 Reasoning</div>
            <div class="field">
              <label>Reasoning Budget</label>
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
              <label>Reasoning Effort</label>
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
        </div>

        <div class="modal-config-footer">
          <button class="btn-secondary" @click="view = 'list'">Cancel</button>
          <button class="btn-load" style="width:auto; padding: 6px 24px;" @click="loadWithConfig">
            ⬆ Load Model
          </button>
        </div>
      </template>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { allModels, loadedModel, selectedModel, modelLoading, loadingModel } from '../stores/selectedModel'
import { modelDisplayNames, modelMeta, groups } from '../stores/groups'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig, loadModelConfig, type ModelConfig } from '../stores/config'
import type { ModelFile } from '../stores/selectedModel'

const search = ref('')
const view = ref<'list' | 'config'>('list')
const configModel = ref<ModelFile | null>(null)
const tempCfg = ref<ModelConfig | null>(null)
const emit = defineEmits(['close'])

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

async function onModelClick(e: MouseEvent, model: ModelFile) {
  if (e.altKey) {
    configModel.value = model
    tempCfg.value = { ...await loadModelConfig(model.path) }
    view.value = 'config'
  } else {
    selectModel(model)
  }
}

async function loadWithConfig() {
  if (!configModel.value || !tempCfg.value) return
  loadingModel.value = configModel.value
  selectedModel.value = configModel.value
  emit('close')
  modelLoading.value = true
  const config = await loadConfig()
  try {
    await invoke('load_model', {
      llamaPath: config.llamaPath,
      modelPath: configModel.value.path,
      gpuLayers: Number(tempCfg.value.gpuOffload),
      contextLength: Number(tempCfg.value.contextLength),
      cpuThreads: Number(tempCfg.value.cpuThreads),
      evalBatch: Number(tempCfg.value.evalBatch),
      physicalBatch: Number(tempCfg.value.physicalBatch),
      flashAttention: tempCfg.value.flashAttention,
      specType: tempCfg.value.specType,
      draftModelPath: tempCfg.value.draftModelPath ?? '',
      maxDraftTokens: Number(tempCfg.value.maxDraftTokens),
      draftProbability: Number(tempCfg.value.draftProbability),
      kCacheQuant: tempCfg.value.kCacheQuant,
      vCacheQuant: tempCfg.value.vCacheQuant,
      port: Number(config.port),
      host: tempCfg.value.host ?? '127.0.0.1',
      alias: tempCfg.value.alias ?? '',
      threadsHttp: Number(tempCfg.value.threadsHttp ?? 2),
      noWarmup: tempCfg.value.noWarmup ?? false,
      sleepIdle: Number(tempCfg.value.sleepIdle ?? -1),
      reasoningPreserve: tempCfg.value.reasoningPreserve ?? false,
      fit: tempCfg.value.fit ?? 'on',
      reasoningBudget: Number(tempCfg.value.reasoningBudget ?? -1),
      reasoningEffort: tempCfg.value.reasoningEffort ?? 'default',
      parallel: Number(tempCfg.value.parallel ?? 1),
      mlock: tempCfg.value.mlock ?? false,
      mmap: tempCfg.value.mmap ?? false,
      kvUnified: tempCfg.value.kvUnified ?? false,
      nCpuMoe: Number(tempCfg.value.nCpuMoe ?? 0),
    })
  } catch (e) {
    modelLoading.value = false
  }
}

async function selectModel(model: ModelFile) {
  loadingModel.value = model
  selectedModel.value = model
  emit('close')
  
  modelLoading.value = true
  const config = await loadConfig()
  const modelCfg = await loadModelConfig(model.path)

  try {
    await invoke('load_model', {
      llamaPath: config.llamaPath,
      modelPath: model.path,
      gpuLayers: Number(modelCfg.gpuOffload ?? 65),
      contextLength: Number(modelCfg.contextLength ?? 100352),
      cpuThreads: Number(modelCfg.cpuThreads ?? 12),
      evalBatch: Number(modelCfg.evalBatch ?? 2048),
      physicalBatch: Number(modelCfg.physicalBatch ?? 512),
      flashAttention: modelCfg.flashAttention ?? true,
      specType: modelCfg.specType ?? 'None',
      draftModelPath: modelCfg.draftModelPath ?? '',
      maxDraftTokens: Number(modelCfg.maxDraftTokens ?? 2),
      draftProbability: Number(modelCfg.draftProbability ?? 0.75),
      kCacheQuant: modelCfg.kCacheQuant ?? 'Q8_0',
      vCacheQuant: modelCfg.vCacheQuant ?? 'Q8_0',
      port: Number(config.port ?? 8080),
      host: modelCfg.host ?? '127.0.0.1',
      alias: modelCfg.alias ?? '',
      threadsHttp: Number(modelCfg.threadsHttp ?? 2),
      noWarmup: modelCfg.noWarmup ?? false,
      sleepIdle: Number(modelCfg.sleepIdle ?? -1),
      reasoningPreserve: modelCfg.reasoningPreserve ?? false,
      fit: modelCfg.fit ?? 'on',
      reasoningBudget: Number(modelCfg.reasoningBudget ?? -1),
      reasoningEffort: modelCfg.reasoningEffort ?? 'default',
      parallel: Number(modelCfg.parallel ?? 1),
      mlock: modelCfg.mlock ?? false,
      mmap: modelCfg.mmap ?? false,
      kvUnified: modelCfg.kvUnified ?? false,
      nCpuMoe: Number(modelCfg.nCpuMoe ?? 0),
    })
  } catch (e) {
    modelLoading.value = false
    console.error(e)
  }
}

async function eject() {
  await invoke('stop_model')
  loadedModel.value = null
  modelLoading.value = false
  emit('close')
}
</script>
