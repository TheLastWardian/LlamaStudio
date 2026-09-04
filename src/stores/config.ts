import { ref } from 'vue'
import { load, type Store } from '@tauri-apps/plugin-store'

const STORE_FILE = 'config.json'

let storePromise: Promise<Store> | null = null

function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true })
  }
  return storePromise
}

export type SpecKind = 'mtp' | 'simple' | 'draftMtp' | 'dflash' | 'eagle3' | 'dspark'

export const SPEC_KINDS: readonly SpecKind[] = ['mtp', 'simple', 'draftMtp', 'dflash', 'eagle3', 'dspark']

export interface DraftParams {
  maxDraftTokens: number
  minDraftTokens: number
  probability: number
  splitProbability: number
  kCacheQuant: string
  vCacheQuant: string
}

export const defaultDraftParams: DraftParams = {
  maxDraftTokens: 2,
  minDraftTokens: 0,
  probability: 0.75,
  splitProbability: 0.10,
  kCacheQuant: 'F16',
  vCacheQuant: 'F16',
}

export function activeSpecKind(cfg: { specType: string; draftSpecType: string }): SpecKind {
  if (cfg.specType === 'MTP') return 'mtp'
  switch (cfg.draftSpecType) {
    case 'mtp': return 'draftMtp'
    case 'dflash': return 'dflash'
    case 'eagle3': return 'eagle3'
    case 'dspark': return 'dspark'
    default: return 'simple'
  }
}

export interface ModelConfig {
  contextLength: number
  gpuOffload: number
  cpuThreads: number
  evalBatch: number
  physicalBatch: number
  flashAttention: boolean
  specType: string
  draftSpecType: string
  draftParams: Record<SpecKind, DraftParams>
  dflashNgramK4v: boolean
  ngramK4vSizeN: number
  ngramK4vSizeM: number
  ngramK4vMinHits: number
  ngramMod: boolean
  ngramModNMatch: number
  ngramModNMin: number
  ngramModNMax: number
  ngramCache: boolean
  kCacheQuant: string
  vCacheQuant: string
  cacheReuse: number
  ctxCheckpoints: number
  checkpointMinStep: number
  reasoning: string
  reasoningBudget: string
  reasoningBudgetCustom: number
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
  specType: 'None',
  draftSpecType: 'simple',
  draftParams: {
    mtp: { ...defaultDraftParams },
    simple: { ...defaultDraftParams },
    draftMtp: { ...defaultDraftParams },
    dflash: { ...defaultDraftParams },
    eagle3: { ...defaultDraftParams },
    dspark: { ...defaultDraftParams },
  },
  dflashNgramK4v: false,
  ngramK4vSizeN: 12,
  ngramK4vSizeM: 48,
  ngramK4vMinHits: 1,
  ngramMod: false,
  ngramModNMatch: 24,
  ngramModNMin: 48,
  ngramModNMax: 64,
  ngramCache: false,
  kCacheQuant: 'Q8_0',
  vCacheQuant: 'Q8_0',
  cacheReuse: 0,
  ctxCheckpoints: 32,
  checkpointMinStep: 8192,
  reasoning: 'auto',
  reasoningBudget: '-1',
  reasoningBudgetCustom: 2048,
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

type LegacyDraftFields = {
  maxDraftTokens?: number
  minDraftTokens?: number
  draftProbability?: number
  draftSplitProbability?: number
  draftKCacheQuant?: string
  draftVCacheQuant?: string
}

export async function loadModelConfig(modelPath: string): Promise<ModelConfig> {
  const store = await getStore()
  const key = 'model:' + modelPath.replace(/[\\/]/g, '_')
  const saved = await store.get<Partial<ModelConfig> & LegacyDraftFields>(key)
  const cfg: ModelConfig = { ...modelDefaults, ...(saved ?? {}) }

  const hasLegacy = !saved?.draftParams
  const legacy: DraftParams = {
    maxDraftTokens: saved?.maxDraftTokens ?? modelDefaults.draftParams.mtp.maxDraftTokens,
    minDraftTokens: saved?.minDraftTokens ?? modelDefaults.draftParams.mtp.minDraftTokens,
    probability: saved?.draftProbability ?? modelDefaults.draftParams.mtp.probability,
    splitProbability: saved?.draftSplitProbability ?? modelDefaults.draftParams.mtp.splitProbability,
    kCacheQuant: saved?.draftKCacheQuant ?? modelDefaults.draftParams.mtp.kCacheQuant,
    vCacheQuant: saved?.draftVCacheQuant ?? modelDefaults.draftParams.mtp.vCacheQuant,
  }
  const active = activeSpecKind(cfg)

  const params = {} as Record<SpecKind, DraftParams>
  for (const kind of SPEC_KINDS) {
    const sp = saved?.draftParams?.[kind]
    if (sp) {
      params[kind] = { ...modelDefaults.draftParams[kind], ...sp }
    } else if (hasLegacy && kind === active) {
      params[kind] = { ...legacy }
    } else {
      params[kind] = { ...modelDefaults.draftParams[kind] }
    }
  }
  cfg.draftParams = params

  return cfg
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
  cudaGraphOpt: string
  logVerbosity: number
  port: number
  minimizeToTray: boolean
  language: 'en' | 'es'
}

const defaults: AppConfig = {
  modelsPath: '',
  llamaPath: '',
  cudaGraphOpt: '',
  logVerbosity: 3,
  port: 8080,
  minimizeToTray: false,
  language: 'en',
}

export const appConfig = ref<AppConfig>({ ...defaults })

export async function loadConfig(): Promise<AppConfig> {
  const store = await getStore()
  appConfig.value = {
    modelsPath: await store.get<string>('modelsPath') ?? defaults.modelsPath,
    llamaPath: await store.get<string>('llamaPath') ?? defaults.llamaPath,
    cudaGraphOpt: await store.get<string>('cudaGraphOpt') ?? defaults.cudaGraphOpt,
    logVerbosity: await store.get<number>('logVerbosity') ?? defaults.logVerbosity,
    port: await store.get<number>('port') ?? defaults.port,
    minimizeToTray: await store.get<boolean>('minimizeToTray') ?? defaults.minimizeToTray,
    language: await store.get<'en' | 'es'>('language') ?? defaults.language,
  }
  return { ...appConfig.value }
}

export async function saveConfig(config: AppConfig): Promise<void> {
  const store = await getStore()
  await store.set('modelsPath', config.modelsPath)
  await store.set('llamaPath', config.llamaPath)
  await store.set('cudaGraphOpt', config.cudaGraphOpt)
  await store.set('logVerbosity', config.logVerbosity)
  await store.set('port', config.port)
  await store.set('minimizeToTray', config.minimizeToTray)
  await store.set('language', config.language)
  await store.save()
  appConfig.value = { ...config }
}
