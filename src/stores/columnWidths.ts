import { reactive } from 'vue'
import { load } from '@tauri-apps/plugin-store'

const STORE_FILE = 'columnWidths.json'

export const columnWidths = reactive<Record<string, number>>({})

export async function loadColumnWidths() {
  const store = await load(STORE_FILE, { autoSave: true })
  const saved = await store.get<Record<string, number>>('columnWidths') ?? {}
  Object.assign(columnWidths, saved)
}

export async function saveColumnWidths() {
  const store = await load(STORE_FILE, { autoSave: true })
  await store.set('columnWidths', { ...columnWidths })
  await store.save()
}
