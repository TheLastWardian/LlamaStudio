import { describe, it, expect } from 'vitest'
import { decideSlot } from './slots'
import type { ModelFile } from '../stores/selectedModel'

const mk = (path: string): ModelFile => ({ name: path, path, arch: 'qwen' } as ModelFile)
const A = mk('a.gguf')
const B = mk('b.gguf')
const C = mk('c.gguf')

const base = {
  ports: [8080, 8081] as number[],
  onSlotsFull: 'replace_first' as const,
}

describe('decideSlot', () => {
  it('1. reload in-place: auto o manual con el mismo puerto repite su puerto', () => {
    expect(decideSlot({ ...base, modelPath: A.path, portMode: 'auto', serverPort: 0, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 8080 })
    expect(decideSlot({ ...base, modelPath: A.path, portMode: 'manual', serverPort: 8080, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 8080 })
  })

  it('1b. reload a manual OTRO puerto: mueve (ejecta la propia instancia + auto-config)', () => {
    expect(decideSlot({ ...base, modelPath: A.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 7000, evict: [8080], autoConfigureIndex: 1 })
  })

  it('1c. reload mover a manual ocupado por otro + replace_first: ejecta la propia y al ocupante', () => {
    expect(decideSlot({ ...base, modelPath: A.path, portMode: 'manual', serverPort: 8081, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'load', port: 8081, evict: [8080, 8081] })
  })

  it('1d. reload mover a manual ocupado por otro + ask: modal al ocupante, propia se ejecta igual', () => {
    expect(decideSlot({ ...base, onSlotsFull: 'ask', modelPath: A.path, portMode: 'manual', serverPort: 8081, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'ask-replace', candidates: [8081], manualPort: 8081, evict: [8080] })
  })

  it('2. auto con slot libre: primer puerto configurado sin modelo', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'auto', serverPort: 0, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 8081 })
  })

  it('3. auto full + replace_first: ejecta slot 0 y carga en su puerto', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'auto', serverPort: 0, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'load', port: 8080, evict: [8080] })
  })

  it('4. auto full + ask: candidatos = todos los cargados (orden por puerto), sin manualPort', () => {
    expect(decideSlot({ ...base, onSlotsFull: 'ask', modelPath: C.path, portMode: 'auto', serverPort: 0, loadedModels: { 8081: B, 8080: A } }))
      .toEqual({ kind: 'ask-replace', candidates: [8080, 8081] })
  })

  it('5. manual caso A (puerto ocupado por otro) + replace_first: silencioso en ese puerto', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'manual', serverPort: 8081, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'load', port: 8081, evict: [8081] })
  })

  it('6. manual caso A + ask: modal restringido al ocupante, con manualPort', () => {
    expect(decideSlot({ ...base, onSlotsFull: 'ask', modelPath: C.path, portMode: 'manual', serverPort: 8081, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'ask-replace', candidates: [8081], manualPort: 8081 })
  })

  it('7. manual caso B con espacio, puerto nuevo: auto-config del primer slot libre j>=1', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 7000, autoConfigureIndex: 1 })
  })

  it('8. manual caso B con espacio, puerto ya en config: sin auto-config', () => {
    expect(decideSlot({ ...base, ports: [8080, 7000], modelPath: C.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8080: A } }))
      .toEqual({ kind: 'load', port: 7000 })
  })

  it('9. manual caso B con espacio, solo libre el slot 0: sin auto-config (sagrado)', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8081: B } }))
      .toEqual({ kind: 'load', port: 7000 })
  })

  it('10. manual caso B sin espacio + replace_first: ejecta slot 0, carga en el manual', () => {
    expect(decideSlot({ ...base, modelPath: C.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'load', port: 7000, evict: [8080] })
  })

  it('11. manual caso B sin espacio + ask: cualquiera de los cargados, manualPort', () => {
    expect(decideSlot({ ...base, onSlotsFull: 'ask', modelPath: C.path, portMode: 'manual', serverPort: 7000, loadedModels: { 8080: A, 8081: B } }))
      .toEqual({ kind: 'ask-replace', candidates: [8080, 8081], manualPort: 7000 })
  })
})
