<template>
  <div class="modal-backdrop">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-header-left">
          <span class="modal-title">{{ t('modal.filter') }}</span>
        </div>
        <button class="modal-close" @click="$emit('close')">{{ t('modal.close') }}</button>
      </div>
      <div class="modal-content">
        <div class="modal-models">
          <div
            v-for="model in filteredModels"
            :key="model.path"
            class="model-option"
            :class="{ selected: selectedModelPath === model.path }"
            @click="selectedModelPath = model.path"
          >
            <span class="model-name">{{ model.name }}</span>
            <span class="model-size">{{ formatSize(model.size_bytes) }}</span>
          </div>
        </div>
        <div class="modal-footer">
          <div class="modal-currently-loaded">
            <span class="modal-currently-loaded-label">{{ t('modal.currentlyLoaded') }}</span>
            <span class="modal-currently-loaded-name">{{ loadedModel?.name }}</span>
          </div>
          <div class="modal-actions">
            <button class="btn-secondary" @click="$emit('close')">{{ t('modal.back') }}</button>
            <button 
              class="btn-load" 
              :disabled="!selectedModelPath"
              @click="handleLoad"
            >
              {{ t('modal.load') }}
            </button>
            <button class="btn-eject" @click="handleEject">{{ t('modal.eject') }}</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, modelLoading, loadedModel, loadingModel, loadedModelConfig } from '../stores/selectedModel'
import { loadConfig, loadModelConfig, saveModelConfig, type ModelConfig } from '../stores/config'
import { allModels } from '../stores/selectedModel'
import { t } from '../i18n'

const emit = defineEmits(['close', 'loaded'])

const search = ref('')
const selectedModelPath = ref('')

const filteredModels = computed(() => {
  const q = search.value.toLowerCase()
  return allModels.value.filter(m => m.name.toLowerCase().includes(q))
})

function formatSize(bytes: number): string {
  return (bytes / 1024 / 1024 / 1024).toFixed(1) + ' GB'
}

async function handleLoad() {
  if (!selectedModelPath.value) return
  const model = allModels.value.find(m => m.path === selectedModelPath.value)
  if (!model) return
  
  emit('close')
  emit('loaded', model)
}

async function handleEject() {
  await invoke('stop_model')
  loadedModel.value = null
  modelLoading.value = false
}
</script>
