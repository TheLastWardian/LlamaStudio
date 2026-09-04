import type { ModelFile } from '../stores/selectedModel'

// KV bytes per token per head per tensor. Q4_0/Q8_0 block sizes include
// scale bytes, so they are slightly above the naive 0.5/1.0.
const KV_BYTES: Record<string, number> = { F32: 4, F16: 2, Q8_0: 1.0625, Q4_0: 0.5625 }
const GIB = 1024 ** 3

export interface VramEstimate {
  weightsGiB: number
  kvGiB: number
  ssmGiB: number
  runtimeGiB: number
  draftGiB: number
  mmprojGiB: number
  totalGiB: number
}

function parseList(s: string | null | undefined): number[] {
  return (s ?? '').split(',').map(n => Number(n)).filter(n => Number.isFinite(n))
}

function headDimOf(m: ModelFile): number {
  if (m.key_length > 0) return m.key_length
  return m.head_count > 0 && m.embedding_length > 0 ? m.embedding_length / m.head_count : 0
}

// KV cache for `nLayers` layers. Models with per-layer geometry (gemma4:
// head_count_kv + sliding_window_pattern arrays) give every offloaded layer a
// KV cache: full context for full-attention layers, sliding window (measured
// window + 512 ubatch) for SWA layers, each with its own head count/dim.
function kvCacheBytes(m: ModelFile, ctx: number, kCache: string, vCache: string, nLayers: number, perLayer: boolean): number {
  const bytesKV = (KV_BYTES[kCache] ?? 2) + (KV_BYTES[vCache] ?? 2)
  if (nLayers <= 0 || ctx <= 0) return 0
  if (perLayer) {
    const kvList = parseList(m.head_count_kv_list).filter(n => n > 0)
    const swaList = m.sliding_window_pattern ? parseList(m.sliding_window_pattern) : []
    if (kvList.length === 0) return 0
    let total = 0
    for (let i = 0; i < nLayers; i++) {
      const kvHeads = kvList[i % kvList.length]
      const isSwa = swaList.length > 0 && swaList[i % swaList.length] === 1
      const headDim = isSwa && m.key_length_swa > 0 ? m.key_length_swa : headDimOf(m)
      if (kvHeads <= 0 || headDim <= 0) continue
      const cells = isSwa && m.sliding_window > 0 ? Math.min(ctx, m.sliding_window + 512) : ctx
      total += cells * kvHeads * headDim * bytesKV
    }
    return total
  }
  const kvHeads = m.head_count_kv > 0 ? m.head_count_kv : m.head_count
  const headDim = headDimOf(m)
  if (kvHeads <= 0 || headDim <= 0) return 0
  return nLayers * ctx * kvHeads * headDim * bytesKV
}

export function estimateVram(
  model: ModelFile | null | undefined,
  opts: {
    ngl: number | null | undefined
    ctx: number | null | undefined
    kCache: string
    vCache: string
    draftKCache?: string
    draftVCache?: string
    specDraftMax?: number
    nCpuMoe?: number
    nParallel?: number
    specMtp?: boolean
    draftModel?: ModelFile | null
    mmprojSizeBytes?: number
  }
): VramEstimate | null {
  if (!model) return null
  const layers = model.layer_count > 0 ? model.layer_count : 999
  const ngl = Math.max(0, Math.min(opts.ngl ?? 0, layers))
  const ctx = Math.max(0, opts.ctx ?? 0)

  let weights = model.size_bytes * (ngl / layers)
  const nCpuMoe = Math.max(0, Math.min(opts.nCpuMoe ?? 0, model.expert_count))
  if (model.is_moe && model.expert_count > 0 && nCpuMoe > 0 && headDimOf(model) > 0 && model.embedding_length > 0) {
    const ffn = model.expert_feed_forward_length > 0 ? model.expert_feed_forward_length : model.feed_forward_length
    if (ffn > 0) {
      const emb = model.embedding_length
      const attn = 2 * emb * headDimOf(model) * (model.head_count + model.head_count_kv)
      const expert = model.expert_count * 3 * emb * ffn
      const frac = expert / (expert + attn)
      weights = model.size_bytes * (ngl / layers) * (1 - frac * nCpuMoe / model.expert_count)
    }
  }

  const specMtp = !!opts.specMtp && ngl > 0
  // MTP can be an extra layer embedded in the main file (qwen35) or a
  // separate assistant model loaded with -md (gemma4); only the latter is
  // passed as draftModel.
  const draft = opts.draftModel && opts.draftModel.path && opts.draftModel.path !== model.path ? opts.draftModel : null
  // Per-layer mode needs a real multi-element head_count_kv array (a scalar
  // value lands in head_count_kv_list as a single entry) AND every offloaded
  // layer to own a KV cache (full_attention_interval <= 1); hybrids (qwen35:
  // interval 4) keep the one-full-layer-per-interval path.
  const perLayer = parseList(model.head_count_kv_list).filter(n => n > 0).length > 1
    && model.full_attention_interval <= 1

  // Growing KV cache. Hybrids without per-layer data keep one full-attention
  // layer per `full_attention_interval`; per-layer models allocate for every
  // offloaded layer (SWA layers only keep their window).
  let kv = 0
  if (perLayer) {
    kv = kvCacheBytes(model, ctx, opts.kCache, opts.vCache, Math.min(ngl, layers), true)
  } else {
    const interval = model.full_attention_interval > 0 ? model.full_attention_interval : 1
    const nFull = ngl > 0 ? Math.floor(ngl / interval) : 0
    kv = kvCacheBytes(model, ctx, opts.kCache, opts.vCache, nFull, false)
  }
  // Embedded MTP allocates one extra single-layer KV cache with its own quant.
  if (specMtp && !draft) {
    kv += kvCacheBytes(model, ctx, opts.draftKCache ?? 'F16', opts.draftVCache ?? 'F16', 1, perLayer)
  }

  // SSM recurrent state (hybrids only): allocated per offloaded layer, scaled
  // by n_parallel and by the recurrent sequence slots, which follow the spec
  // draft n_max (measured: n_max 3 → 3 rs, n_max 5 → 5 rs); without spec
  // decoding there is 1 slot.
  let ssm = 0
  if (ngl > 0 && model.ssm_state_size > 0 && model.ssm_inner_size > 0) {
    const nParallel = Math.max(1, opts.nParallel ?? 1)
    const rsSeq = specMtp ? Math.max(1, opts.specDraftMax ?? 1) : 1
    ssm = ngl * model.ssm_state_size * model.ssm_inner_size * 4 * nParallel * rsSeq
  }

  // Separate MTP/draft model (-md): its weights load fully on GPU
  // (--spec-draft-ngl 99), plus a compute buffer (~0.7x its weights, measured
  // gemma4-assistant 147/225 MiB). Its KV is shared with the main layers when
  // all its layers are shared (gemma4-assistant: shared_kv_layers = 4/4).
  let draftBytes = 0
  if (draft && ngl > 0) {
    draftBytes = draft.size_bytes * 1.7
    const dLayers = draft.layer_count > 0 ? draft.layer_count : 1
    if (draft.shared_kv_layers < dLayers) {
      draftBytes += kvCacheBytes(draft, ctx, opts.draftKCache ?? 'F16', opts.draftVCache ?? 'F16', dLayers,
        parseList(draft.head_count_kv_list).some(n => n > 0))
    }
  }

  // Runtime: compute buffers scale with active compute, not total weights:
  // ~5% of GPU weights per context for dense models (861/15,839 MiB), ~2%
  // for MoE (360/19,400 MiB for A3B; gemma4 326/13,213 MiB). Embedded MTP
  // spec decoding adds a second one (x2); a separate draft model brings its
  // own buffer, already counted in draftGiB. Plus CUDA context/driver
  // overhead (~0.5 GiB).
  const computeRate = model.is_moe ? 0.02 : 0.05
  const runtime = Math.max(0.5 * GIB, weights * computeRate * (specMtp && !draft ? 2 : 1)) + 0.5 * GIB

  // Vision encoder (mmproj): model + its compute buffer (measured 1,290.09
  // MiB for a 1,139.46 MiB file).
  const mmproj = (opts.mmprojSizeBytes ?? 0) > 0 ? (opts.mmprojSizeBytes ?? 0) * 1.13 : 0

  return {
    weightsGiB: weights / GIB,
    kvGiB: kv / GIB,
    ssmGiB: ssm / GIB,
    runtimeGiB: runtime / GIB,
    draftGiB: draftBytes / GIB,
    mmprojGiB: mmproj / GIB,
    totalGiB: (weights + kv + ssm + runtime + draftBytes + mmproj) / GIB,
  }
}
