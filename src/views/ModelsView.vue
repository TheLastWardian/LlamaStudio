<template>
  <div class="models-layout">
    <div class="models-main">
      <div class="topbar">
        <span class="topbar-title">Models</span>
        <input class="search-box" v-model="search" placeholder="Filter models... (Ctrl+F)" />
      </div>
      <div class="content">
        <div class="model-list-header">
          <div 
            v-for="col in columns" 
            :key="col.key"
            class="col-header"
            :style="{ width: col.width + 'px' }"
          >
            {{ col.label }}
            <div class="col-resize-handle" @mousedown="startColResize($event, col)"></div>
          </div>
        </div>
        <div 
          v-for="model in filteredModels" 
          :key="model.path"
          class="model-row"
          :class="{ selected: selectedModel?.path === model.path }"
          @click="selectedModel = model"
        >
          <div class="col-cell" :style="{ width: columns[0].width + 'px' }">
            <span class="tag" :class="'arch-' + model.arch">{{ model.arch || '?' }}</span>
          </div>
          <div class="col-cell" :style="{ width: columns[1].width + 'px' }">
            <span class="badge">{{ model.params || '?' }}</span>
          </div>
          <div class="col-cell" :style="{ width: columns[2].width + 'px' }">{{ model.publisher }}</div>
          <div class="col-cell" :style="{ width: columns[3].width + 'px' }">{{ model.name }}</div>
          <div class="col-cell" :style="{ width: columns[4].width + 'px' }">
            <span class="tag quant">{{ model.name.split('-').pop()?.replace('.gguf','').replace('.GGUF','') }}</span>
          </div>
          <div class="col-cell" :style="{ width: columns[5].width + 'px' }">{{ (model.size_bytes / 1024 / 1024 / 1024).toFixed(1) }} GB</div>
          <div class="col-cell" :style="{ width: columns[6].width + 'px' }">-</div>
          <div class="col-cell" :style="{ width: columns[7].width + 'px' }">···</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { loadConfig } from '../stores/config'
import { selectedModel, allModels } from '../stores/selectedModel'

interface ModelFile {
  name: string
  publisher: string
  model_family: string
  size_bytes: number
  path: string
  arch: string
  params: string
  max_context: number
}

const models = ref<ModelFile[]>([])
const search = ref('')

const filteredModels = computed(() => {
  if (!search.value) return models.value
  return models.value.filter(m => 
    m.name.toLowerCase().includes(search.value.toLowerCase()) ||
    m.publisher.toLowerCase().includes(search.value.toLowerCase())
  )
})

const columns = ref([
  { key: 'arch', label: 'Arch', width: 100 },
  { key: 'params', label: 'Params', width: 70 },
  { key: 'publisher', label: 'Publisher', width: 110 },
  { key: 'llm', label: 'LLM', width: 400 },
  { key: 'quant', label: 'Quant', width: 90 },
  { key: 'size', label: 'Size', width: 80 },
  { key: 'modified', label: 'Modified', width: 100 },
  { key: 'actions', label: 'Actions', width: 60 },
])

function startColResize(e: MouseEvent, col: { key: string; label: string; width: number }) {
  e.preventDefault()
  e.stopPropagation()
  const startX = e.clientX
  const startWidth = col.width

  function onMove(e: MouseEvent) {
    col.width = Math.max(50, startWidth + (e.clientX - startX))
  }

  function onUp() {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }

  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

onMounted(async () => {
  const config = await loadConfig()
  models.value = await invoke('scan_models', {
    modelsPath: config.modelsPath
  })
  if (models.value.length > 0 && !selectedModel.value) {
    selectedModel.value = models.value[0]
  }
  allModels.value = models.value
})
</script>
