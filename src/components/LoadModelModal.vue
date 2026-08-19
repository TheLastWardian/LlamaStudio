<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal">
      <div class="modal-header">
        <input class="search-box" v-model="search" placeholder="Type to filter models..." autofocus />
        <button class="modal-close" @click="$emit('close')">✕</button>
      </div>

      <!-- Modelo cargado actualmente -->
      <div v-if="loadedModel" class="modal-section-title">Currently Loaded (1)</div>
      <div v-if="loadedModel" class="modal-model-row loaded" @click="$emit('close')">
        <span class="modal-model-name">{{ modelDisplayNames[loadedModel.path] || loadedModel.name }}</span>
        <span class="tag quant">{{ loadedModel.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
        <span class="tag" style="background:#1a2a1a; color:#4af54a;">GGUF</span>
        <span class="badge" v-if="loadedModel.name.toLowerCase().includes('mtp')">MTP</span>
        <div style="flex:1"></div>
        <button class="btn-eject" @click.stop="eject">⏏ Eject</button>
      </div>

      <!-- Lista de modelos -->
      <div class="modal-section-title">Your Models</div>
      <div class="modal-model-list">
        <div 
          v-for="model in filteredModels" 
          :key="model.path"
          class="modal-model-row"
          :class="{ active: loadedModel?.path === model.path }"
          @click="selectModel(model)"
        >
          <span class="modal-model-name">{{ modelDisplayNames[model.path] || model.name }}</span>
          <span class="tag quant">{{ model.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
          <span class="tag" style="background:#2a2a2a; color:#888;">GGUF</span>
          <div style="flex:1"></div>
          <span style="color:#555; font-size:11px;">{{ (model.size_bytes / 1024 / 1024 / 1024).toFixed(2) }} GB</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { allModels, loadedModel, selectedModel, modelLoading } from '../stores/selectedModel'
import { modelDisplayNames } from '../stores/groups'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig, loadModelConfig } from '../stores/config'

const search = ref('')
const emit = defineEmits(['close'])

const filteredModels = computed(() => {
  return allModels.value.filter(m => 
    m.path !== loadedModel.value?.path &&
    m.name.toLowerCase().includes(search.value.toLowerCase())
  )
})

async function selectModel(model: typeof allModels.value[0]) {
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
