import type { ModelFile } from '../stores/selectedModel'

const KV_BYTES: Record<string, number> = { F32: 4, F16: 2, Q8_0: 1, Q4_0: 0.5 }
const GIB = 1024 ** 3
const BUFFER_BYTES = 0.2 * GIB

export interface VramEstimate {
  weightsGiB: number
  kvGiB: number
  bufferGiB: number
  totalGiB: number
}

export function estimateVram(
  model: ModelFile | null | undefined,
  opts: { ngl: number | null | undefined; ctx: number | null | undefined; kCache: string; vCache: string }
): VramEstimate | null {
  if (!model) return null
  const layers = model.layer_count > 0 ? model.layer_count : 999
  const ngl = Math.max(0, Math.min(opts.ngl ?? 0, layers))
  const ctx = Math.max(0, opts.ctx ?? 0)

  const weights = model.size_bytes * (ngl / layers)

  let kv = 0
  if (ngl > 0 && model.head_count > 0 && model.embedding_length > 0) {
    const headDim = model.embedding_length / model.head_count
    const kvHeads = model.head_count_kv > 0 ? model.head_count_kv : model.head_count
    kv = ctx * layers * kvHeads * headDim * ((KV_BYTES[opts.kCache] ?? 2) + (KV_BYTES[opts.vCache] ?? 2))
  }

  return {
    weightsGiB: weights / GIB,
    kvGiB: kv / GIB,
    bufferGiB: BUFFER_BYTES / GIB,
    totalGiB: (weights + kv + BUFFER_BYTES) / GIB,
  }
}
