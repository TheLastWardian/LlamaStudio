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
            :data-group-id="section.group.id"
            @contextmenu="onRightClickGroup($event, section.group.id)"
          >
            <span>📁 {{ section.group.name }}</span>
            <span style="color:#555; font-size:11px;">{{ section.models.length }} models</span>
          </div>
          <div v-else-if="groupedModels.length > 1" class="group-header ungrouped" data-group-id="ungrouped">
            <span>Ungrouped</span>
            <span style="color:#555; font-size:11px;">{{ section.models.length }} models</span>
          </div>

          <!-- Modelos -->
          <div
            v-for="model in section.models"
            :key="model.path"
            class="model-row"
            :data-group-id="section.group?.id ?? 'ungrouped'"
            :data-model-path="model.path"
            :class="{ selected: selectedModel?.path === model.path, 'drop-target': dropTarget !== null && dragging?.modelPath !== model.path && dropModelPath === model.path }"
            @click="selectedModel = model"
            @contextmenu="onRightClickModel($event, model.path)"
            @mousedown="onModelMouseDown($event, model.path)"
          >
            <div class="col-cell" :style="{ width: columns[0].width + 'px' }">
              <span class="pin-icon" @click.stop="handleTogglePin(model.path)" :class="{ pinned: modelMeta[model.path]?.pinned }">📌</span>
              <span class="tag" :class="'arch-' + model.arch">{{ model.arch || '?' }}</span>
            </div>
            <div class="col-cell" :style="{ width: columns[1].width + 'px' }">
              <span class="badge">{{ model.params || '?' }}</span>
            </div>
            <div class="col-cell" :style="{ width: columns[2].width + 'px' }">{{ model.publisher }}</div>
            <div class="col-cell" :style="{ width: columns[3].width + 'px' }">
              <template v-if="renamingPath === model.path">
                <input class="rename-input" v-model="renameValue" @keyup.enter="confirmRename" @keyup.escape="cancelRename" @blur="confirmRename" @click.stop />
              </template>
              <template v-else>
                {{ modelDisplayNames[model.path] || model.name }}
              </template>
            </div>
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
        <div class="ctx-item" @click="startRename(ctxMenu.modelPath!)">✏ Rename</div>
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
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { selectedModel, allModels } from '../stores/selectedModel'
import { loadConfig } from '../stores/config'
import { groups, modelMeta, modelDisplayNames, createGroup, deleteGroup, moveModelToGroup, togglePin, saveGroups } from '../stores/groups'
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
  listVersion.value
  const filtered = models.value.filter(m =>
    m.name.toLowerCase().includes(search.value.toLowerCase()) ||
    m.publisher.toLowerCase().includes(search.value.toLowerCase())
  )

  const result: { group: { id: string, name: string } | null, models: ModelFile[] }[] = []

  // Grupos definidos
  const sortedGroups = [...groups.value].sort((a, b) => a.order - b.order)
  for (const group of sortedGroups) {
    const groupModels = filtered
      .filter(m => modelMeta[m.path]?.groupId === group.id)
      .sort((a, b) => {
        const aPinned = modelMeta[a.path]?.pinned ?? false
        const bPinned = modelMeta[b.path]?.pinned ?? false
        if (aPinned !== bPinned) return aPinned ? -1 : 1
        return (modelMeta[a.path]?.order ?? 0) - (modelMeta[b.path]?.order ?? 0)
      })
    result.push({ group, models: groupModels })
  }

  // Ungrouped
  const ungrouped = filtered
    .filter(m => !modelMeta[m.path]?.groupId || !groups.value.find(g => g.id === modelMeta[m.path]?.groupId))
    .sort((a, b) => {
      const aPinned = modelMeta[a.path]?.pinned ?? false
      const bPinned = modelMeta[b.path]?.pinned ?? false
      if (aPinned !== bPinned) return aPinned ? -1 : 1
      return (modelMeta[a.path]?.order ?? 0) - (modelMeta[b.path]?.order ?? 0)
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

const dragging = ref<{ modelPath: string, x: number, y: number } | null>(null)
const dropTarget = ref<string | null>(null)
const dropModelPath = ref<string | null>(null)
const listVersion = ref(0)
const renamingPath = ref<string | null>(null)
const renameValue = ref('')

function startRename(modelPath: string) {
  closeCtxMenu()
  setTimeout(() => {
    renamingPath.value = modelPath
    renameValue.value = modelDisplayNames[modelPath] || models.value.find(m => m.path === modelPath)?.name || ''
  }, 50)
}

watch(renamingPath, async (newPath) => {
  if (newPath) {
    await nextTick()
    await nextTick()
    const input = document.querySelector('.rename-input') as HTMLInputElement
    if (input) {
      input.focus()
      input.select()
    }
  }
})

function confirmRename() {
  if (renamingPath.value) {
    if (renameValue.value.trim()) {
      modelDisplayNames[renamingPath.value] = renameValue.value.trim()
    } else {
      delete modelDisplayNames[renamingPath.value]
    }
    saveGroups()
  }
  renamingPath.value = null
}

function cancelRename() {
  renamingPath.value = null
}

function onModelMouseDown(e: MouseEvent, modelPath: string) {
  if (e.button !== 0) return
  if (renamingPath.value) return
  e.preventDefault()
  
  const startX = e.clientX
  const startY = e.clientY
  let started = false

  function onMove(e: MouseEvent) {
    if (!started && Math.abs(e.clientX - startX) + Math.abs(e.clientY - startY) > 8) {
      started = true
    }
    if (started) {
      dragging.value = { modelPath, x: e.clientX, y: e.clientY }
      const el = document.elementFromPoint(e.clientX, e.clientY)
      const groupEl = el?.closest('[data-group-id]')
      dropTarget.value = groupEl?.getAttribute('data-group-id') ?? null
      dropModelPath.value = groupEl?.getAttribute('data-model-path') ?? null
    }
  }

  function cleanup() {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
    window.removeEventListener('mouseleave', onUp)
  }

  function onUp() {
      if (started && dragging.value) {
      const gId = dropTarget.value === 'ungrouped' ? null : dropTarget.value

      if (dropModelPath.value && dropModelPath.value !== modelPath) {
        const targetGroupId = modelMeta[dropModelPath.value]?.groupId ?? null
        
        // Asignar al grupo del target
        const newMeta = { ...modelMeta }
        if (!newMeta[modelPath]) newMeta[modelPath] = { groupId: targetGroupId, pinned: false, order: 999 }
        else newMeta[modelPath] = { ...newMeta[modelPath], groupId: targetGroupId }

        // Obtener todos los modelos del grupo destino incluyendo el que movemos
        const groupModels = models.value
          .filter(m => (newMeta[m.path]?.groupId ?? null) === targetGroupId)
          .sort((a, b) => (newMeta[a.path]?.order ?? 0) - (newMeta[b.path]?.order ?? 0))

        // Si no está en la lista aún, agregarlo
        if (!groupModels.find(m => m.path === modelPath)) {
          const draggedModel = models.value.find(m => m.path === modelPath)
          if (draggedModel) groupModels.push(draggedModel)
        }

        const fromIdx = groupModels.findIndex(m => m.path === modelPath)
        const toIdx = groupModels.findIndex(m => m.path === dropModelPath.value)



        if (fromIdx !== -1 && toIdx !== -1) {
          const arr = [...groupModels]
          const [moved] = arr.splice(fromIdx, 1)
          arr.splice(toIdx, 0, moved)
          arr.forEach((m, i) => {
            newMeta[m.path] = { ...(newMeta[m.path] ?? { groupId: targetGroupId, pinned: false }), order: i }
          })
        }
        Object.assign(modelMeta, newMeta)
        saveGroups()
        listVersion.value++
      } else if (dropTarget.value !== null) {
        moveModelToGroup(modelPath, gId)
      }
    }

    dragging.value = null
    dropTarget.value = null
    dropModelPath.value = null
    cleanup()
  }

  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
  window.addEventListener('mouseleave', onUp)
}
</script>

<style scoped>
.rename-input {
  background: #2a2a2a;
  border: 1px solid #5a8af5;
  border-radius: 3px;
  color: #fff;
  padding: 2px 6px;
  font-size: 12px;
  width: 100%;
  outline: none;
}
</style>
