<template>
  <div class="models-layout" @click="closeCtxMenu">
    <div class="models-main">
      <div class="topbar">
        <span class="topbar-title">Models</span>
        <input class="search-box" v-model="search" placeholder="Filter models... (Ctrl+F)" />
      </div>

      <div class="content" @contextmenu="onRightClickEmpty">
        <!-- Header -->
        <div class="model-list-header">
          <div v-for="col in columns" :key="col.key" class="col-header" :style="{ width: col.width + 'px' }">
            {{ col.label }}
            <div class="col-resize-handle" @mousedown="startColResize($event, col)"></div>
          </div>
        </div>

        <!-- Grupos y modelos -->
        <template v-for="section in groupedModels" :key="section.group?.id ?? 'ungrouped'">
          <!-- Header de grupo -->
          <div 
            v-if="section.group"
            class="group-header"
            @contextmenu="onRightClickGroup($event, section.group.id)"
          >
            <span>📁 {{ section.group.name }}</span>
            <span style="color:#555; font-size:11px;">{{ section.models.length }} models</span>
          </div>
          <div v-else-if="groupedModels.length > 1" class="group-header ungrouped">
            <span>Ungrouped</span>
            <span style="color:#555; font-size:11px;">{{ section.models.length }} models</span>
          </div>

          <!-- Modelos -->
          <div
            v-for="model in section.models"
            :key="model.path"
            class="model-row"
            :class="{ selected: selectedModel?.path === model.path }"
            @click="selectedModel = model"
            @contextmenu="onRightClickModel($event, model.path)"
          >
            <div class="col-cell" :style="{ width: columns[0].width + 'px' }">
              <span class="pin-icon" @click.stop="handleTogglePin(model.path)" :class="{ pinned: modelMeta[model.path]?.pinned }">📌</span>
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
            <div class="col-cell" :style="{ width: columns[5].width + 'px' }">{{ formatSize(model.size_bytes) }}</div>
            <div class="col-cell" :style="{ width: columns[6].width + 'px' }">-</div>
            <div class="col-cell" :style="{ width: columns[7].width + 'px' }">···</div>
          </div>
        </template>
      </div>
    </div>

    <!-- Context Menu -->
    <div 
      v-if="ctxMenu"
      class="ctx-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      @click.stop
    >
      <!-- Menú para área vacía -->
      <template v-if="ctxMenu.type === 'empty'">
        <div class="ctx-item" @click="startCreateGroup">
          <template v-if="!showGroupInput">
            ➕ Create Group
          </template>
          <template v-else>
            <input 
              class="ctx-input" 
              v-model="newGroupName" 
              placeholder="Group name..."
              @keyup.enter="confirmCreateGroup"
              @keyup.escape="closeCtxMenu"
              autofocus
            />
            <button class="ctx-btn" @click="confirmCreateGroup">✓</button>
          </template>
        </div>
      </template>

      <!-- Menú para modelo -->
      <template v-if="ctxMenu.type === 'model' && ctxMenu.modelPath">
        <div class="ctx-item" @click="handleTogglePin(ctxMenu.modelPath!)">
          {{ modelMeta[ctxMenu.modelPath!]?.pinned ? '📌 Unpin' : '📌 Pin' }}
        </div>
        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="moveToGroupMenu = !moveToGroupMenu">
          📁 Move to group ▶
        </div>
        <div v-if="moveToGroupMenu" class="ctx-submenu">
          <div class="ctx-item" @click="handleMoveToGroup(ctxMenu.modelPath!, null)">
            — Ungrouped
          </div>
          <div 
            v-for="g in groups" 
            :key="g.id"
            class="ctx-item"
            @click="handleMoveToGroup(ctxMenu.modelPath!, g.id)"
          >
            📁 {{ g.name }}
          </div>
        </div>
      </template>

      <!-- Menú para grupo -->
      <template v-if="ctxMenu.type === 'group' && ctxMenu.groupId">
        <div class="ctx-item danger" @click="handleDeleteGroup(ctxMenu.groupId!)">
          🗑 Delete Group
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, allModels } from '../stores/selectedModel'
import { loadConfig } from '../stores/config'
import { groups, modelMeta, createGroup, deleteGroup, moveModelToGroup, togglePin } from '../stores/groups'
import type { ModelFile } from '../stores/selectedModel'

const search = ref('')
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

// Context menu
const ctxMenu = ref<{ x: number, y: number, type: 'model' | 'empty' | 'group', modelPath?: string, groupId?: string } | null>(null)
const showGroupInput = ref(false)
const newGroupName = ref('')
const moveToGroupMenu = ref(false)

const models = ref<ModelFile[]>([])

function formatSize(bytes: number): string {
  return (bytes / 1024 / 1024 / 1024).toFixed(1) + ' GB'
}

onMounted(async () => {
  const config = await loadConfig()
  models.value = await invoke('scan_models', { modelsPath: config.modelsPath })
  allModels.value = models.value
  if (models.value.length > 0 && !selectedModel.value) {
    selectedModel.value = models.value[0]
  }
})

// Modelos agrupados y ordenados
const groupedModels = computed(() => {
  const filtered = models.value.filter(m =>
    m.name.toLowerCase().includes(search.value.toLowerCase()) ||
    m.publisher.toLowerCase().includes(search.value.toLowerCase())
  )

  const result: { group: { id: string, name: string } | null, models: ModelFile[] }[] = []

  // Grupos definidos
  const sortedGroups = [...groups.value].sort((a, b) => a.order - b.order)
  for (const group of sortedGroups) {
    const groupModels = filtered
      .filter(m => modelMeta.value[m.path]?.groupId === group.id)
      .sort((a, b) => {
        const aPinned = modelMeta.value[a.path]?.pinned ?? false
        const bPinned = modelMeta.value[b.path]?.pinned ?? false
        if (aPinned !== bPinned) return aPinned ? -1 : 1
        return (modelMeta.value[a.path]?.order ?? 0) - (modelMeta.value[b.path]?.order ?? 0)
      })
    result.push({ group, models: groupModels })
  }

  // Ungrouped
  const ungrouped = filtered
    .filter(m => !modelMeta.value[m.path]?.groupId || !groups.value.find(g => g.id === modelMeta.value[m.path]?.groupId))
    .sort((a, b) => {
      const aPinned = modelMeta.value[a.path]?.pinned ?? false
      const bPinned = modelMeta.value[b.path]?.pinned ?? false
      if (aPinned !== bPinned) return aPinned ? -1 : 1
      return 0
    })
  result.push({ group: null, models: ungrouped })

  return result
})

// Context menu handlers
function onRightClickEmpty(e: MouseEvent) {
  e.preventDefault()
  ctxMenu.value = { x: e.clientX, y: e.clientY, type: 'empty' }
  moveToGroupMenu.value = false
}

function onRightClickModel(e: MouseEvent, modelPath: string) {
  e.preventDefault()
  e.stopPropagation()
  ctxMenu.value = { x: e.clientX, y: e.clientY, type: 'model', modelPath }
  moveToGroupMenu.value = false
}

function onRightClickGroup(e: MouseEvent, groupId: string) {
  e.preventDefault()
  e.stopPropagation()
  ctxMenu.value = { x: e.clientX, y: e.clientY, type: 'group', groupId }
  moveToGroupMenu.value = false
}

function closeCtxMenu() {
  ctxMenu.value = null
  showGroupInput.value = false
  moveToGroupMenu.value = false
}

function startCreateGroup() {
  showGroupInput.value = true
  newGroupName.value = ''
}

function confirmCreateGroup() {
  if (newGroupName.value.trim()) {
    createGroup(newGroupName.value.trim())
  }
  closeCtxMenu()
}

function handleDeleteGroup(id: string) {
  deleteGroup(id)
  closeCtxMenu()
}

function handleMoveToGroup(modelPath: string, groupId: string | null) {
  moveModelToGroup(modelPath, groupId)
  closeCtxMenu()
}

function handleTogglePin(modelPath: string) {
  togglePin(modelPath)
  closeCtxMenu()
}

function startColResize(e: MouseEvent, col: typeof columns.value[0]) {
  e.preventDefault()
  e.stopPropagation()
  const startX = e.clientX
  const startWidth = col.width
  function onMove(e: MouseEvent) { col.width = Math.max(50, startWidth + (e.clientX - startX)) }
  function onUp() { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp) }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

const dragModel = ref<string | null>(null)
const dragGroup = ref<string | null>(null)
const dragOverGroup = ref<string | null>(null)

function onModelDragStart(e: DragEvent, modelPath: string) {
  dragModel.value = modelPath
  dragGroup.value = null
  e.dataTransfer!.effectAllowed = 'move'
}

function onGroupDragStart(e: DragEvent, groupId: string) {
  dragGroup.value = groupId
  dragModel.value = null
  e.dataTransfer!.effectAllowed = 'move'
}

function onDragOverGroup(e: DragEvent, groupId: string | null) {
  e.preventDefault()
  dragOverGroup.value = groupId ?? 'ungrouped'
}

function onDragLeave() {
  dragOverGroup.value = null
}

function onDropOnGroup(e: DragEvent, groupId: string | null) {
  e.preventDefault()
  dragOverGroup.value = null

  if (dragModel.value) {
    moveModelToGroup(dragModel.value, groupId)
    dragModel.value = null
  } else if (dragGroup.value && groupId !== null) {
    const from = groups.value.findIndex(g => g.id === dragGroup.value)
    const to = groups.value.findIndex(g => g.id === groupId)
    if (from !== -1 && to !== -1) {
      const arr = [...groups.value]
      const [moved] = arr.splice(from, 1)
      arr.splice(to, 0, moved)
      arr.forEach((g, i) => g.order = i)
      groups.value = arr
      saveGroups()
    }
    dragGroup.value = null
  }
}

function onDropOnModel(e: DragEvent, targetPath: string, groupId: string | null) {
  e.preventDefault()
  dragOverGroup.value = null

  if (dragModel.value && dragModel.value !== targetPath) {
    moveModelToGroup(dragModel.value, groupId)
    
    const meta = modelMeta.value
    if (!meta[dragModel.value]) meta[dragModel.value] = { groupId, pinned: false, order: 0 }
    if (!meta[targetPath]) meta[targetPath] = { groupId, pinned: false, order: 0 }
    
    const fromOrder = meta[dragModel.value].order
    const toOrder = meta[targetPath].order
    meta[dragModel.value].order = toOrder
    meta[targetPath].order = fromOrder
    saveGroups()
    dragModel.value = null
  }
}
</script>
