import type { ModelFile } from '../stores/selectedModel'

const KV_BYTES: Record<string, number> = { F32: 4, F16: 2, Q8_0: 1.0625, Q4_0: 0.5625 }
const GIB = 1024 ** 3
const BUFFER_BYTES = 0.2 * GIB

export interface VramEstimate {
  weightsGiB: number
  kvGiB: number
  ssmGiB: number
  bufferGiB: number
  totalGiB: number
}

export function estimateVram(
  model: ModelFile | null | undefined,
  opts: { ngl: number | null | undefined; ctx: number | null | undefined; kCache: string; vCache: string; nCpuMoe?: number }
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

  const interval = model.full_attention_interval > 0 ? model.full_attention_interval : 1
  const nFull = ngl > 0 ? Math.ceil(ngl / interval) : 0
  let kv = 0
  if (headDim > 0 && model.head_count_kv > 0 && nFull > 0) {
    const kvHeads = model.head_count_kv > 0 ? model.head_count_kv : model.head_count
    kv = nFull * ctx * kvHeads * headDim * ((KV_BYTES[opts.kCache] ?? 2) + (KV_BYTES[opts.vCache] ?? 2))
  }

  const nSsm = Math.max(0, ngl - nFull)
  let ssm = 0
  if (nSsm > 0 && model.ssm_state_size > 0 && model.ssm_inner_size > 0) {
    ssm = nSsm * model.ssm_state_size * model.ssm_inner_size * 4
  }

  return {
    weightsGiB: weights / GIB,
    kvGiB: kv / GIB,
    ssmGiB: ssm / GIB,
    bufferGiB: BUFFER_BYTES / GIB,
    totalGiB: (weights + kv + ssm + BUFFER_BYTES) / GIB,
  }
}
