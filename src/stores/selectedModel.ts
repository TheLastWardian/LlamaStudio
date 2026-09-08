import { ref, computed } from 'vue'
import { appConfig } from './config'

export interface ModelFile {
  name: string
  publisher: string
  model_family: string
  size_bytes: number
  path: string
  arch: string
  params: string
  max_context: number
  layer_count: number
  embedding_length: number
  head_count: number
  head_count_kv: number
  key_length: number
  sliding_window: number
  sliding_window_pattern: string
  key_length_swa: number
  head_count_kv_list: string
  shared_kv_layers: number
  full_attention_interval: number
  ssm_state_size: number
  ssm_inner_size: number
  feed_forward_length: number
  expert_feed_forward_length: number
  is_moe: boolean
  expert_count: number
  expert_used_count: number
  supports_thinking: boolean
  supports_effort: boolean
  supported_effort_levels: string[]
  mmproj_paths: string[]
  is_draft: boolean
}

export interface LogLine { time: string; level: string; msg: string }
export interface GenState { prefill: number | null; tokens: number | null }

export const selectedModel = ref<ModelFile | null>(null)
export const allModels = ref<ModelFile[]>([])
export const modelLoading = ref(false)

// Estado por puerto
export const loadedModels = ref<Record<number, ModelFile>>({})
export const loadingModelFull = ref<{ port: number; model: ModelFile } | null>(null)
export const serverLogsByPort = ref<Record<number, LogLine[]>>({})
export const launchCmdByPort = ref<Record<number, string>>({})
export const launchSpecByPort = ref<Record<number, string>>({})
export const genState = ref<Record<number, GenState>>({})

// El modelo activo = el cargado en chatPort (decisión 3)
export const activeLoadedModel = computed(() => loadedModels.value[appConfig.value.chatPort] ?? null)
export const activeGen = (): GenState => genState.value[appConfig.value.chatPort] ?? { prefill: null, tokens: null }

export function portOfModel(model: ModelFile): number | null {
  for (const [p, m] of Object.entries(loadedModels.value)) {
    if (m.path === model.path) return Number(p)
  }
  return null
}

export function setLoaded(port: number, model: ModelFile) {
  loadedModels.value = { ...loadedModels.value, [port]: model }
  if (!genState.value[port]) genState.value = { ...genState.value, [port]: { prefill: null, tokens: null } }
}

// Marca el inicio de carga y garantiza que `genState[port]` exista desde el
// primer evento (un print_timing temprano no se pierde por `if (gs)`)
export function setLoading(port: number, model: ModelFile) {
  loadingModelFull.value = { port, model }
  if (!genState.value[port]) genState.value = { ...genState.value, [port]: { prefill: null, tokens: null } }
}

export function removeLoaded(port: number) {
  const next = { ...loadedModels.value }
  delete next[port]
  loadedModels.value = next
  const gs = { ...genState.value }
  delete gs[port]
  genState.value = gs
  const cmd = { ...launchCmdByPort.value }; delete cmd[port]; launchCmdByPort.value = cmd
  const spec = { ...launchSpecByPort.value }; delete spec[port]; launchSpecByPort.value = spec
}
