import { load, type Store } from '@tauri-apps/plugin-store'

const STORE_FILE = 'config.json'

let storePromise: Promise<Store> | null = null

function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true })
  }
  return storePromise
}

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
  cacheReuse: number
  ctxCheckpoints: number
  checkpointMinStep: number
  reasoningBudget: string
  reasoningEffort: string
  draftModelPath: string
  host: string
  alias: string
  threadsHttp: number
  noWarmup: boolean
  sleepIdle: number
  reasoningPreserve: boolean
  fit: string
  parallel: number
  mlock: boolean
  mmap: boolean
  kvUnified: boolean
  kvOffload: boolean
  cacheRam: number
  nCpuMoe: number
  expertsPerToken: number
  visionEnabled: boolean
  mmprojPath: string
  seed: number
  temp: number
  topP: number
  topK: number
  minP: number
  repeatPenalty: number
}

const modelDefaults: ModelConfig = {
  contextLength: 4096,
  gpuOffload: 0,
  cpuThreads: 0,
  evalBatch: 2048,
  physicalBatch: 512,
  flashAttention: true,
  specType: 'MTP',
  maxDraftTokens: 2,
  draftProbability: 0.75,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
  cacheReuse: 0,
  ctxCheckpoints: 32,
  checkpointMinStep: 8192,
  reasoningBudget: '-1',
  reasoningEffort: 'default',
  draftModelPath: '',
  host: '127.0.0.1',
  alias: '',
  threadsHttp: 2,
  noWarmup: false,
  sleepIdle: -1,
  reasoningPreserve: false,
  fit: 'on',
  parallel: 1,
  mlock: false,
  mmap: false,
  kvUnified: false,
  kvOffload: false,
  cacheRam: 0,
  nCpuMoe: 0,
  expertsPerToken: 0,
  visionEnabled: false,
  mmprojPath: '',
  seed: -1,
  temp: 0.8,
  topP: 0.95,
  topK: 40,
  minP: 0.05,
  repeatPenalty: 1.0,
}

export async function loadModelConfig(modelPath: string): Promise<ModelConfig> {
  const store = await getStore()
  const key = 'model:' + modelPath.replace(/[\\/]/g, '_')
  const saved = await store.get<Partial<ModelConfig>>(key)
  return { ...modelDefaults, ...saved }
}

export async function saveModelConfig(modelPath: string, config: ModelConfig): Promise<void> {
  const store = await getStore()
  const key = 'model:' + modelPath.replace(/[\\/]/g, '_')
  await store.set(key, config)
  await store.save()
}

export interface AppConfig {
  modelsPath: string
  llamaPath: string
  port: number
  minimizeToTray: boolean
  language: 'en' | 'es'
}

const defaults: AppConfig = {
  modelsPath: '',
  llamaPath: '',
  port: 8080,
  minimizeToTray: false,
  language: 'en',
}

export async function loadConfig(): Promise<AppConfig> {
  const store = await getStore()
  return {
    modelsPath: await store.get<string>('modelsPath') ?? defaults.modelsPath,
    llamaPath: await store.get<string>('llamaPath') ?? defaults.llamaPath,
    port: await store.get<number>('port') ?? defaults.port,
    minimizeToTray: await store.get<boolean>('minimizeToTray') ?? defaults.minimizeToTray,
    language: await store.get<'en' | 'es'>('language') ?? defaults.language,
  }
}

export async function saveConfig(config: AppConfig): Promise<void> {
  const store = await getStore()
  await store.set('modelsPath', config.modelsPath)
  await store.set('llamaPath', config.llamaPath)
  await store.set('port', config.port)
  await store.set('minimizeToTray', config.minimizeToTray)
  await store.set('language', config.language)
  await store.save()
}
