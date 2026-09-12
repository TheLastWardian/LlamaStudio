import { invoke } from '@tauri-apps/api/core'
import { loadConfig, saveModelConfig, setPortAt, type AppConfig, type ModelConfig, activeSpecKind, defaultDraftParams, numOrDefault } from '../stores/config'
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
// haya en el puerto de carga mismo. Devuelve el puerto donde quedó cargado y los puertos
// que se detuvieron (el llamador debe limpiarlos del estado frontend: su kill suprime
// llama-exited, ver stop_flag en Rust).
export async function executeLoad(prep: PreparedLoad, port: number, chosenPort?: number): Promise<{ port: number; stopped: number[] }> {
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
  const resolvedThreads = numOrDefault(cfg.cpuThreads, 0) > 0 ? numOrDefault(cfg.cpuThreads, 0) : await invoke<number>('get_cpu_threads')
  const dp = cfg.draftParams[activeSpecKind(cfg)] ?? defaultDraftParams
  await invoke('load_model', {
    llamaPath: config.llamaPath,
    cudaGraphOpt: config.cudaGraphOpt ?? '',
    logVerbosity: Number(config.logVerbosity ?? 3),
    modelPath: model.path,
    gpuLayers: numOrDefault(cfg.gpuOffload, 999),
    contextLength: numOrDefault(cfg.contextLength, 4096),
    cpuThreads: resolvedThreads,
    evalBatch: numOrDefault(cfg.evalBatch, 2048),
    physicalBatch: numOrDefault(cfg.physicalBatch, 512),
    flashAttention: cfg.flashAttention,
    specType: cfg.specType,
    draftSpecType: cfg.draftSpecType,
    draftModelPath: cfg.draftModelPath,
    maxDraftTokens: numOrDefault(dp.maxDraftTokens, 2),
    minDraftTokens: numOrDefault(dp.minDraftTokens, 0),
    draftProbability: numOrDefault(dp.probability, 0.75),
    draftSplitProbability: numOrDefault(dp.splitProbability, 0.10),
    dflashNgramK4v: cfg.dflashNgramK4v,
    ngramK4vSizeN: numOrDefault(cfg.ngramK4vSizeN, 12),
    ngramK4vSizeM: numOrDefault(cfg.ngramK4vSizeM, 48),
    ngramK4vMinHits: numOrDefault(cfg.ngramK4vMinHits, 1),
    ngramMod: cfg.ngramMod,
    ngramModNMatch: numOrDefault(cfg.ngramModNMatch, 24),
    ngramModNMin: numOrDefault(cfg.ngramModNMin, 48),
    ngramModNMax: numOrDefault(cfg.ngramModNMax, 64),
    ngramCache: cfg.ngramCache,
    kCacheQuant: cfg.kCacheQuant,
    vCacheQuant: cfg.vCacheQuant,
    draftKCacheQuant: dp.kCacheQuant,
    draftVCacheQuant: dp.vCacheQuant,
    cacheReuse: numOrDefault(cfg.cacheReuse, 0),
    ctxCheckpoints: numOrDefault(cfg.ctxCheckpoints, 32),
    checkpointMinStep: numOrDefault(cfg.checkpointMinStep, 8192),
    port,
    host: cfg.host,
    alias: cfg.alias,
    threadsHttp: numOrDefault(cfg.threadsHttp, 2),
    noWarmup: cfg.noWarmup,
    sleepIdle: numOrDefault(cfg.sleepIdle, -1),
    reasoningPreserve: cfg.reasoningPreserve,
    fit: cfg.fit,
    reasoning: cfg.reasoning,
    reasoningBudget: cfg.reasoningBudget === 'custom' ? Math.max(1, numOrDefault(cfg.reasoningBudgetCustom, 2048)) : numOrDefault(cfg.reasoningBudget, -1),
    reasoningEffort: cfg.reasoningEffort,
    parallel: numOrDefault(cfg.parallel, 1),
    mlock: cfg.mlock,
    mmap: cfg.mmap,
    kvUnified: cfg.kvUnified,
    kvOffload: cfg.kvOffload,
    cacheRam: numOrDefault(cfg.cacheRam, 0),
    nCpuMoe: numOrDefault(cfg.nCpuMoe, 0),
    expertsPerToken: numOrDefault(cfg.expertsPerToken, 0),
    visionEnabled: cfg.visionEnabled,
    mmprojPath: cfg.mmprojPath,
    imageMinTokens: numOrDefault(cfg.imageMinTokens, 0),
    mmprojGpu: cfg.mmprojGpu,
    seed: numOrDefault(cfg.seed, -1),
    temp: numOrDefault(cfg.temp, 0.8),
    topP: numOrDefault(cfg.topP, 0.95),
    topK: numOrDefault(cfg.topK, 40),
    minP: numOrDefault(cfg.minP, 0.05),
    repeatPenalty: numOrDefault(cfg.repeatPenalty, 1.0),
  })
  return { port, stopped: toStop }
}
