import { ref } from 'vue'

export function useReplaceFlow() {
  const showReplace = ref(false)
  const replaceCandidates = ref<number[] | null>(null)
  let resolver: ((port: number | null) => void) | null = null

  function askReplace(candidates: number[]): Promise<number | null> {
    replaceCandidates.value = candidates
    showReplace.value = true
    return new Promise(res => { resolver = res })
  }
  function choose(port: number) {
    showReplace.value = false
    resolver?.(port)
    resolver = null
  }
  function close() {
    showReplace.value = false
    resolver?.(null)
    resolver = null
  }
  return { showReplace, replaceCandidates, askReplace, choose, close }
}
