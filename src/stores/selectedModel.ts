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
}

export const selectedModel = ref<ModelFile | null>(null)

export const allModels = ref<ModelFile[]>([])

export const modelLoading = ref(false)

export const serverLogs = ref<{time: string, level: string, msg: string}[]>([])

export const loadedModel = ref<ModelFile | null>(null)
