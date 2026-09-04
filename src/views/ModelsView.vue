<template>
  <div class="models-layout" @click="closeCtxMenu">
    <div class="models-main">
      <div class="topbar">
        <span class="topbar-title">{{ t('topbar.models') }}</span>
        <input class="search-box" v-model="search" :placeholder="t('models.filter')" />
        <button class="btn-refresh" :disabled="scanning" :title="t('models.refresh')" @click="rescanModels">⟳</button>
      </div>

      <div class="content" @contextmenu="onRightClickEmpty">
        <!-- Header -->
        <div class="model-list-header">
          <div v-for="col in columns" :key="col.key" class="col-header" :style="{ width: col.width + 'px' }">
            {{ t(col.labelKey) }}
            <div class="col-resize-handle" @mousedown="startColResize($event, col)"></div>
          </div>
        </div>

        <!-- Grupos y modelos -->
        <template v-for="section in groupedModels" :key="section.group?.id ?? 'ungrouped'">
          <!-- Header de grupo -->
          <div 
            v-if="section.group"
            class="group-header"
            :class="{ 'drop-group-target': dropGroupTarget === section.group.id }"
            :data-group-id="section.group.id"
            @mousedown="onGroupMouseDown($event, section.group.id)"
            @contextmenu="onRightClickGroup($event, section.group.id)"
          >
            <span class="group-collapse-icon" @click.stop="toggleCollapse(section.group.id)">
              {{ collapsedGroups[section.group.id] ? '▶' : '▼' }}
            </span>
            <span>📁 {{ section.group.name }}</span>
            <span style="color:#555; font-size:11px; margin-left:auto;">{{ section.models.length }} {{ t('models.modelsCount') }}</span>
          </div>
          <div v-else-if="groupedModels.length > 1" class="group-header ungrouped" data-group-id="ungrouped">
            <span class="group-collapse-icon" @click="toggleCollapse('ungrouped')">
              {{ collapsedGroups['ungrouped'] ? '▶' : '▼' }}
            </span>
            <span>{{ t('models.ungrouped') }}</span>
            <span style="color:#555; font-size:11px; margin-left:auto;">{{ section.models.length }} {{ t('models.modelsCount') }}</span>
          </div>

          <!-- Modelos solo si no está colapsado -->
          <template v-if="!collapsedGroups[section.group?.id ?? 'ungrouped']">
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
            <div class="col-cell" :style="{ width: columns[6].width + 'px' }">···</div>
          </div>
          </template>
        </template>
      </div>

      <!-- Models folder footer -->
      <div class="models-footer">
        <span class="models-footer-label">{{ t('models.folderPath') }}</span>
        <span class="models-footer-path" :title="modelsPath">{{ modelsPath }}</span>
        <button class="models-footer-open" :disabled="!modelsPath" @click="openModelsFolder">{{ t('models.openFolder') }}</button>
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
            ➕ {{ t('modelList.createGroup') }}
          </template>
          <template v-else>
            <input 
              class="ctx-input" 
              v-model="newGroupName" 
              :placeholder="t('modelList.groupPlaceholder')"
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
          {{ modelMeta[ctxMenu.modelPath!]?.pinned ? t('modelList.unpin') : t('modelList.pin') }}
        </div>
        <div class="ctx-item" @click="startRename(ctxMenu.modelPath!)">{{ t('modelList.rename') }}</div>
        <div class="ctx-item" @click="revealModelLocation(ctxMenu.modelPath!)">{{ t('modelList.revealInFolder') }}</div>
        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="moveToGroupMenu = !moveToGroupMenu">
          {{ t('modelList.moveToGroup') }}
        </div>
        <div v-if="moveToGroupMenu" class="ctx-submenu">
          <div class="ctx-item" @click="handleMoveToGroup(ctxMenu.modelPath!, null)">
            — {{ t('contextMenu.ungrouped') }}
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
          {{ t('modelList.deleteGroup') }}
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { revealItemInDir, openPath } from '@tauri-apps/plugin-opener'
import { selectedModel, allModels } from '../stores/selectedModel'
import { loadConfig } from '../stores/config'
import { groups, modelMeta, modelDisplayNames, createGroup, deleteGroup, moveModelToGroup, togglePin, saveGroups, collapsedGroups } from '../stores/groups'
import type { ModelFile } from '../stores/selectedModel'
import { t } from '../i18n'

const search = ref('')
const columns = ref([
  { key: 'arch', labelKey: 'models.arch', width: 100 },
  { key: 'params', labelKey: 'models.params', width: 70 },
  { key: 'publisher', labelKey: 'models.publisher', width: 110 },
  { key: 'llm', labelKey: 'models.llm', width: 400 },
  { key: 'quant', labelKey: 'models.quant', width: 90 },
  { key: 'size', labelKey: 'models.size', width: 80 },
  { key: 'actions', labelKey: 'models.actions', width: 60 },
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

const scanning = ref(false)
const modelsPath = ref('')

async function rescanModels() {
  if (scanning.value) return
  scanning.value = true
  try {
    const config = await loadConfig()
    modelsPath.value = config.modelsPath
    models.value = await invoke('scan_models', { modelsPath: config.modelsPath })
    allModels.value = models.value
    if (models.value.length > 0 && !selectedModel.value) {
      selectedModel.value = models.value[0]
    }
    listVersion.value++
  } finally {
    scanning.value = false
  }
}

onMounted(() => { rescanModels() })

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

async function revealModelLocation(modelPath: string) {
  closeCtxMenu()
  try {
    await revealItemInDir(modelPath)
  } catch (e) {
    console.error('Failed to reveal model in folder:', e)
  }
}

async function openModelsFolder() {
  if (!modelsPath.value) return
  try {
    await openPath(modelsPath.value)
  } catch (e) {
    console.error('Failed to open models folder:', e)
  }
}

const activeCleanups: Array<() => void> = []

function startColResize(e: MouseEvent, col: typeof columns.value[0]) {
  e.preventDefault()
  e.stopPropagation()
  const startX = e.clientX
  const startWidth = col.width
  function onMove(e: MouseEvent) { col.width = Math.max(50, startWidth + (e.clientX - startX)) }
  function cleanup() {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', cleanup)
    const i = activeCleanups.indexOf(cleanup)
    if (i !== -1) activeCleanups.splice(i, 1)
  }
  activeCleanups.push(cleanup)
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', cleanup)
}

const dragging = ref<{ modelPath: string } | null>(null)
const dropTarget = ref<string | null>(null)
const dropModelPath = ref<string | null>(null)
const listVersion = ref(0)
const renamingPath = ref<string | null>(null)
const renameValue = ref('')
const draggingGroup = ref<string | null>(null)
const dropGroupTarget = ref<string | null>(null)

function toggleCollapse(groupId: string) {
  collapsedGroups[groupId] = !collapsedGroups[groupId]
}

function onGroupMouseDown(e: MouseEvent, groupId: string) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement
  if (target.classList.contains('group-collapse-icon')) return

  const startX = e.clientX
  const startY = e.clientY
  let started = false

  function onMove(e: MouseEvent) {
    if (!started && Math.abs(e.clientX - startX) + Math.abs(e.clientY - startY) > 8) {
      started = true
      draggingGroup.value = groupId
    }
    if (started) {
      const el = document.elementFromPoint(e.clientX, e.clientY)
      const groupEl = el?.closest('[data-group-id]')
      const targetId = groupEl?.getAttribute('data-group-id')
      dropGroupTarget.value = targetId && targetId !== groupId ? targetId : null
    }
  }

  function cleanup() {
    if (started && draggingGroup.value && dropGroupTarget.value) {
      const from = groups.value.findIndex(g => g.id === draggingGroup.value)
      const to = groups.value.findIndex(g => g.id === dropGroupTarget.value)
      if (from !== -1 && to !== -1) {
        const arr = [...groups.value]
        const [moved] = arr.splice(from, 1)
        arr.splice(to, 0, moved)
        arr.forEach((g, i) => g.order = i)
        groups.value = arr
        saveGroups()
      }
    }
    draggingGroup.value = null
    dropGroupTarget.value = null
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', cleanup)
    const i = activeCleanups.indexOf(cleanup)
    if (i !== -1) activeCleanups.splice(i, 1)
  }

  activeCleanups.push(cleanup)
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', cleanup)
}

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
      dragging.value = { modelPath }
      const el = document.elementFromPoint(e.clientX, e.clientY)
      const groupEl = el?.closest('[data-group-id]')
      dropTarget.value = groupEl?.getAttribute('data-group-id') ?? null
      dropModelPath.value = groupEl?.getAttribute('data-model-path') ?? null
    }
  }

  function cleanup() {
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
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', cleanup)
    document.removeEventListener('mouseleave', cleanup)
    const i = activeCleanups.indexOf(cleanup)
    if (i !== -1) activeCleanups.splice(i, 1)
  }

  activeCleanups.push(cleanup)
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', cleanup)
  document.addEventListener('mouseleave', cleanup)
}

onUnmounted(() => {
  for (const c of [...activeCleanups]) c()
})
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

.group-collapse-icon {
  cursor: pointer;
  color: #555;
  font-size: 10px;
  margin-right: 6px;
  user-select: none;
}

.group-collapse-icon:hover {
  color: #aaa;
}

.drop-group-target {
  background: #1a2a3a !important;
  border: 1px dashed #5a8af5;
}
</style>
