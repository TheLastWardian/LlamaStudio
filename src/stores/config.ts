import { load } from '@tauri-apps/plugin-store'

const STORE_FILE = 'config.json'

export interface ModelConfig {
  contextLength: number
  gpuOffload: number
  cpuThreads: number
  evalBatch: number
  physicalBatch: number
  flashAttention: boolean
  specType: string
  maxDraftTokens: number
  draftProbability: number
  kCacheQuant: string
  vCacheQuant: string
  reasoningBudget: string
  reasoningEffort: string
  draftModelPath: string
}

const modelDefaults: ModelConfig = {
  contextLength: 100352,
  gpuOffload: 65,
  cpuThreads: 12,
  evalBatch: 2048,
  physicalBatch: 512,
  flashAttention: true,
  specType: 'MTP',
  maxDraftTokens: 2,
  draftProbability: 0.75,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
  reasoningBudget: '-1',
  reasoningEffort: 'default',
  draftModelPath: '',
  minimizeToTray: false,
}

export async function loadModelConfig(modelPath: string): Promise<ModelConfig> {
  const store = await load(STORE_FILE, { autoSave: true })
  const key = 'model:' + modelPath.replace(/[\\/]/g, '_')
  const saved = await store.get<ModelConfig>(key)
  return saved ?? { ...modelDefaults }
}

export async function saveModelConfig(modelPath: string, config: ModelConfig): Promise<void> {
  const store = await load(STORE_FILE, { autoSave: true })
  const key = 'model:' + modelPath.replace(/[\\/]/g, '_')
  await store.set(key, config)
  await store.save()
}

export interface AppConfig {
  modelsPath: string
  llamaPath: string
  port: number
  minimizeToTray: boolean
}

const defaults: AppConfig = {
  modelsPath: 'F:\\Users\\Wardian\\.lmstudio\\models',
  llamaPath: '',
  port: 8080,
  minimizeToTray: false
}

export async function loadConfig(): Promise<AppConfig> {
  const store = await load(STORE_FILE, { autoSave: true })
  return {
    modelsPath: await store.get<string>('modelsPath') ?? defaults.modelsPath,
    llamaPath: await store.get<string>('llamaPath') ?? defaults.llamaPath,
    port: await store.get<number>('port') ?? defaults.port,
    minimizeToTray: await store.get<boolean>('minimizeToTray') ?? defaults.minimizeToTray,
  }
}

export async function saveConfig(config: AppConfig): Promise<void> {
  const store = await load(STORE_FILE, { autoSave: true })
  await store.set('modelsPath', config.modelsPath)
  await store.set('llamaPath', config.llamaPath)
  await store.set('port', config.port)
  await store.set('minimizeToTray', config.minimizeToTray)
  await store.save()
}
