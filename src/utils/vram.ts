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
  totalGiB: number
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
    nCpuMoe?: number
    nParallel?: number
    specMtp?: boolean
  }
): VramEstimate | null {
  if (!model) return null
  const layers = model.layer_count > 0 ? model.layer_count : 999
  const ngl = Math.max(0, Math.min(opts.ngl ?? 0, layers))
  const ctx = Math.max(0, opts.ctx ?? 0)

  const headDim = model.key_length > 0
    ? model.key_length
    : (model.head_count > 0 && model.embedding_length > 0 ? model.embedding_length / model.head_count : 0)

  let weights = model.size_bytes * (ngl / layers)
  const nCpuMoe = Math.max(0, Math.min(opts.nCpuMoe ?? 0, model.expert_count))
  if (model.is_moe && model.expert_count > 0 && nCpuMoe > 0 && headDim > 0 && model.embedding_length > 0) {
    const ffn = model.expert_feed_forward_length > 0 ? model.expert_feed_forward_length : model.feed_forward_length
    if (ffn > 0) {
      const emb = model.embedding_length
      const attn = 2 * emb * headDim * (model.head_count + model.head_count_kv)
      const expert = model.expert_count * 3 * emb * ffn
      const frac = expert / (expert + attn)
      weights = model.size_bytes * (ngl / layers) * (1 - frac * nCpuMoe / model.expert_count)
    }
  }

  const specMtp = !!opts.specMtp && ngl > 0

  // Growing KV cache only lives in full-attention layers. Dense models have
  // full_attention_interval = 0 (or 1) → every offloaded layer. Hybrids
  // (qwen3-next/qwen35, gpt-oss) keep one full-attention layer per `interval`
  // (measured qwen35: 16 layers for 65 offloaded). MTP spec decoding
  // allocates one extra single-layer KV cache.
  const interval = model.full_attention_interval > 0 ? model.full_attention_interval : 1
  const nFull = ngl > 0 ? Math.floor(ngl / interval) : 0
  const kvLayers = nFull + (specMtp ? 1 : 0)
  let kv = 0
  if (headDim > 0 && model.head_count_kv > 0 && kvLayers > 0) {
    const kvHeads = model.head_count_kv > 0 ? model.head_count_kv : model.head_count
    kv = nFull * ctx * kvHeads * headDim * ((KV_BYTES[opts.kCache] ?? 2) + (KV_BYTES[opts.vCache] ?? 2))
    // The MTP draft KV uses its own (independent) quantization.
    if (specMtp) {
      kv += ctx * kvHeads * headDim * ((KV_BYTES[opts.draftKCache ?? 'F16'] ?? 2) + (KV_BYTES[opts.draftVCache ?? 'F16'] ?? 2))
    }
  }

  // SSM recurrent state (hybrids only): the server allocates it per offloaded
  // layer, scaled by n_parallel and its recurrent sequence slots.
  // Measured (qwen35, -np 2): 64 layers x 3 MiB x 2 seqs x 3 rs x ~1.04
  // = 1197 MiB vs llama.cpp's reported 1197 MiB.
  let ssm = 0
  if (ngl > 0 && model.ssm_state_size > 0 && model.ssm_inner_size > 0) {
    const nParallel = Math.max(1, opts.nParallel ?? 1)
    ssm = ngl * model.ssm_state_size * model.ssm_inner_size * 4 * nParallel * 3 * 1.04
  }

  // Runtime: compute buffers (~5% of GPU weights per context; measured
  // 861 MiB / 15,839 MiB at batch 512, MTP adds a second one) plus
  // CUDA context/driver overhead (~0.5 GiB).
  const runtime = Math.max(0.5 * GIB, weights * 0.05 * (specMtp ? 2 : 1)) + 0.5 * GIB

  return {
    weightsGiB: weights / GIB,
    kvGiB: kv / GIB,
    ssmGiB: ssm / GIB,
    runtimeGiB: runtime / GIB,
    totalGiB: (weights + kv + ssm + runtime) / GIB,
  }
}
