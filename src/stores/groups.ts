import { ref, reactive } from 'vue'
import { load } from '@tauri-apps/plugin-store'

export interface Group {
  id: string
  name: string
  order: number
}

export interface ModelMeta {
  groupId: string | null
  pinned: boolean
  order: number
}

const STORE_FILE = 'groups.json'

export const groups = ref<Group[]>([])
export const modelMeta = reactive<Record<string, ModelMeta>>({})
export const modelDisplayNames = reactive<Record<string, string>>({})

export async function loadGroups() {
  const store = await load(STORE_FILE, { autoSave: true })
  groups.value = await store.get<Group[]>('groups') ?? []
  const saved = await store.get<Record<string, ModelMeta>>('modelMeta') ?? {}
  Object.assign(modelMeta, saved)
  const savedNames = await store.get<Record<string, string>>('modelDisplayNames') ?? {}
  Object.assign(modelDisplayNames, savedNames)
}

export async function saveGroups() {
  const store = await load(STORE_FILE, { autoSave: true })
  await store.set('groups', groups.value)
  await store.set('modelMeta', { ...modelMeta })
  await store.set('modelDisplayNames', { ...modelDisplayNames })
  await store.save()
}

export function createGroup(name: string) {
  const id = Date.now().toString()
  groups.value.push({ id, name, order: groups.value.length })
  saveGroups()
  return id
}

export function deleteGroup(id: string) {
  groups.value = groups.value.filter(g => g.id !== id)
  // mover modelos del grupo eliminado a ungrouped
  for (const key in modelMeta) {
    if (modelMeta[key].groupId === id) {
      modelMeta[key].groupId = null
    }
  }
  saveGroups()
}

export function moveModelToGroup(modelPath: string, groupId: string | null) {
  if (!modelMeta[modelPath]) {
    modelMeta[modelPath] = { groupId, pinned: false, order: 0 }
  } else {
    modelMeta[modelPath].groupId = groupId
  }
  saveGroups()
}

export function togglePin(modelPath: string) {
  if (!modelMeta[modelPath]) {
    modelMeta[modelPath] = { groupId: null, pinned: true, order: 0 }
  } else {
    modelMeta[modelPath].pinned = !modelMeta[modelPath].pinned
  }
  saveGroups()
}
