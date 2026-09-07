import type { ModelFile } from '../stores/selectedModel'

export type SlotDecision =
  | { kind: 'load'; port: number; evict?: number[]; autoConfigureIndex?: number }
  | { kind: 'ask-replace'; candidates: number[]; manualPort?: number; evict?: number[] }

export interface DecideSlotInput {
  modelPath: string
  portMode: 'auto' | 'manual'
  serverPort: number // 0 = sin valor (manual requiere > 0; lo valida el llamador)
  loadedModels: Record<number, ModelFile>
  ports: number[] // length = serverCount
  onSlotsFull: 'ask' | 'replace_first'
}

// `evict` = puertos a detener antes de cargar. `executeLoad` deduplica y
// agrega el puerto elegido en el flujo 'ask'. El puerto de carga mismo no va
// en `evict`: Rust lo maneja (mata la instancia que haya en ese puerto).
export function decideSlot({ modelPath, portMode, serverPort, loadedModels, ports, onSlotsFull }: DecideSlotInput): SlotDecision {
  const loadedPorts = Object.keys(loadedModels).map(Number).sort((a, b) => a - b)
  const isLoaded = (port: number) => loadedModels[port] !== undefined

  // 1) Reload in-place: auto o manual con el MISMO puerto → mata+relanza ahí
  const current = loadedPorts.find(p => loadedModels[p].path === modelPath)
  if (current !== undefined && (portMode === 'auto' || serverPort === current)) {
    return { kind: 'load', port: current }
  }
  // Manual a OTRO puerto = "mover": la propia instancia vieja se ejecta
  const own = current // number | undefined
  const evictOwn = own !== undefined ? [own] : []

  // 2) Auto (si el modelo estaba cargado, ya retornó arriba)
  if (portMode === 'auto') {
    for (const port of ports) {
      if (!isLoaded(port)) return { kind: 'load', port }
    }
    // Full: replace_first → slot 0; ask → modal con todos
    if (onSlotsFull === 'replace_first') return { kind: 'load', port: ports[0], evict: [ports[0]] }
    return { kind: 'ask-replace', candidates: loadedPorts }
  }

  // 3) Manual (P validado por el llamador: 1..65535)
  const P = serverPort
  if (isLoaded(P)) {
    // Caso A: ocupado por otro (el propio puerto ya habría retornado in-place)
    if (onSlotsFull === 'replace_first') return { kind: 'load', port: P, evict: [...evictOwn, P] }
    return withEvict({ kind: 'ask-replace', candidates: [P], manualPort: P }, evictOwn)
  }
  // Caso B: P libre
  if (loadedPorts.length < ports.length) {
    if (ports.includes(P)) return withEvict({ kind: 'load', port: P }, evictOwn)
    const j = ports.findIndex((_, i) => i >= 1 && !isLoaded(ports[i]))
    return withEvict(j !== -1 ? { kind: 'load', port: P, autoConfigureIndex: j } : { kind: 'load', port: P }, evictOwn)
  }
  // Caso B sin espacio
  if (onSlotsFull === 'replace_first') return { kind: 'load', port: P, evict: [...evictOwn, ports[0]] }
  return withEvict({ kind: 'ask-replace', candidates: loadedPorts, manualPort: P }, evictOwn)
}

// Solo agrega `evict` si no está vacío (para que `toEqual` sin la key siga pasando)
function withEvict<T extends { kind: string }>(base: T, evict: number[]): T & { evict?: number[] } {
  return evict.length > 0 ? { ...base, evict } : base
}
