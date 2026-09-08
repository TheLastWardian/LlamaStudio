import { invoke } from '@tauri-apps/api/core'
import { loadConfig, saveModelConfig, setPortAt, type AppConfig, type ModelConfig, activeSpecKind, defaultDraftParams } from '../stores/config'
import { loadedModels } from '../stores/selectedModel'
import type { ModelFile } from '../stores/selectedModel'
import { decideSlot, type SlotDecision } from './slots'

export interface PreparedLoad {
  config: AppConfig
  cfg: ModelConfig
  model: ModelFile
  decision: SlotDecision
}

export async function prepareLoad(model: ModelFile, cfg: ModelConfig): Promise<PreparedLoad> {
  const config = await loadConfig()
  const decision = decideSlot({
    modelPath: model.path,
    portMode: cfg.portMode,
    serverPort: cfg.serverPort,
    loadedModels: loadedModels.value,
    ports: config.ports,
    onSlotsFull: config.onSlotsFull,
  })
  return { config, cfg, model, decision }
}

// Ejecuta la decisión: evict (si aplica), auto-config de puerto manual (si aplica) y el load_model.
// `chosenPort` = puerto elegido en el modal (flujo 'ask'); `decision.evict` = puertos
// adicionales (p. ej. la propia instancia vieja al mover un reload). Rust mata lo que
// haya en el puerto de carga mismo. Devuelve el puerto donde quedó cargado.
export async function executeLoad(prep: PreparedLoad, port: number, chosenPort?: number): Promise<number> {
  const { config, cfg, model, decision } = prep
  const toStop = Array.from(new Set([
    ...(decision.evict ?? []),
    ...(chosenPort !== undefined ? [chosenPort] : []),
  ])).filter(p => p !== port)
  for (const p of toStop) {
    await invoke('stop_model', { port: p })
  }
  if (decision.kind === 'load' && decision.autoConfigureIndex !== undefined) {
    await setPortAt(decision.autoConfigureIndex, port)
    await saveModelConfig(model.path, cfg)
  }
  const resolvedThreads = cfg.cpuThreads > 0 ? cfg.cpuThreads : await invoke<number>('get_cpu_threads')
  const dp = cfg.draftParams[activeSpecKind(cfg)] ?? defaultDraftParams
  await invoke('load_model', {
    llamaPath: config.llamaPath,
    cudaGraphOpt: config.cudaGraphOpt ?? '',
    logVerbosity: Number(config.logVerbosity ?? 3),
    modelPath: model.path,
    gpuLayers: cfg.gpuOffload,
    contextLength: cfg.contextLength,
    cpuThreads: resolvedThreads,
    evalBatch: cfg.evalBatch,
    physicalBatch: cfg.physicalBatch,
    flashAttention: cfg.flashAttention,
    specType: cfg.specType,
    draftSpecType: cfg.draftSpecType,
    draftModelPath: cfg.draftModelPath,
    maxDraftTokens: dp.maxDraftTokens,
    minDraftTokens: dp.minDraftTokens,
    draftProbability: dp.probability,
    draftSplitProbability: dp.splitProbability,
    dflashNgramK4v: cfg.dflashNgramK4v,
    ngramK4vSizeN: cfg.ngramK4vSizeN,
    ngramK4vSizeM: cfg.ngramK4vSizeM,
    ngramK4vMinHits: cfg.ngramK4vMinHits,
    ngramMod: cfg.ngramMod,
    ngramModNMatch: cfg.ngramModNMatch,
    ngramModNMin: cfg.ngramModNMin,
    ngramModNMax: cfg.ngramModNMax,
    ngramCache: cfg.ngramCache,
    kCacheQuant: cfg.kCacheQuant,
    vCacheQuant: cfg.vCacheQuant,
    draftKCacheQuant: dp.kCacheQuant,
    draftVCacheQuant: dp.vCacheQuant,
    cacheReuse: cfg.cacheReuse,
    ctxCheckpoints: cfg.ctxCheckpoints,
    checkpointMinStep: cfg.checkpointMinStep,
    port,
    host: cfg.host,
    alias: cfg.alias,
    threadsHttp: cfg.threadsHttp,
    noWarmup: cfg.noWarmup,
    sleepIdle: cfg.sleepIdle,
    reasoningPreserve: cfg.reasoningPreserve,
    fit: cfg.fit,
    reasoning: cfg.reasoning,
    reasoningBudget: cfg.reasoningBudget === 'custom' ? Math.max(1, cfg.reasoningBudgetCustom) : Number(cfg.reasoningBudget),
    reasoningEffort: cfg.reasoningEffort,
    parallel: cfg.parallel,
    mlock: cfg.mlock,
    mmap: cfg.mmap,
    kvUnified: cfg.kvUnified,
    kvOffload: cfg.kvOffload,
    cacheRam: cfg.cacheRam,
    nCpuMoe: cfg.nCpuMoe,
    expertsPerToken: cfg.expertsPerToken,
    visionEnabled: cfg.visionEnabled,
    mmprojPath: cfg.mmprojPath,
    imageMinTokens: cfg.imageMinTokens,
    seed: cfg.seed,
    temp: cfg.temp,
    topP: cfg.topP,
    topK: cfg.topK,
    minP: cfg.minP,
    repeatPenalty: cfg.repeatPenalty,
  })
  return port
}
