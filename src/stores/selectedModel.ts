import { ref } from 'vue'

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
}

export const selectedModel = ref<ModelFile | null>(null)

export const allModels = ref<ModelFile[]>([])

export const modelLoading = ref(false)

export const loadingModel = ref<ModelFile | null>(null)

export const serverLogs = ref<{time: string, level: string, msg: string}[]>([])

export const loadedModel = ref<ModelFile | null>(null)

export const loadedModelConfig = ref<Record<string, any> | null>(null)

export const loadedServerPort = ref<number | null>(null)

export const prefillProgress = ref<number | null>(null)

export const generationTokens = ref<number | null>(null)
