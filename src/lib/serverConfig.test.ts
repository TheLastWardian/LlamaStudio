import { describe, it, expect } from 'vitest'
import { normalizeServerConfig } from './serverConfig'

describe('normalizeServerConfig', () => {
  it('migra config vieja (solo port) a serverCount=1', () => {
    expect(normalizeServerConfig({ port: 8080 })).toEqual({
      serverCount: 1,
      ports: [8080],
      chatPort: 8080,
      onSlotsFull: 'replace_first',
    })
  })

  it('migra config vieja con puerto custom', () => {
    expect(normalizeServerConfig({ port: 9000 }).ports).toEqual([9000])
    expect(normalizeServerConfig({ port: 9000 }).chatPort).toBe(9000)
  })

  it('completa ports faltantes con último+1', () => {
    expect(normalizeServerConfig({ serverCount: 3, ports: [8080] }).ports).toEqual([8080, 8081, 8082])
  })

  it('trunca ports sobrantes al reducir serverCount', () => {
    expect(normalizeServerConfig({ serverCount: 2, ports: [8080, 8081, 8082] }).ports).toEqual([8080, 8081])
  })

  it('chatPort fuera de ports → ports[0]', () => {
    expect(normalizeServerConfig({ serverCount: 2, ports: [8080, 8081], chatPort: 9999 }).chatPort).toBe(8080)
  })

  it('chatPort dentro de ports se respeta', () => {
    expect(normalizeServerConfig({ serverCount: 2, ports: [8080, 8081], chatPort: 8081 }).chatPort).toBe(8081)
  })

  it('serverCount fuera de rango se clamp (0→1, 15→10)', () => {
    expect(normalizeServerConfig({ serverCount: 0 }).serverCount).toBe(1)
    expect(normalizeServerConfig({ serverCount: 15, ports: [8080] }).serverCount).toBe(10)
  })

  it('preserva onSlotsFull explícito', () => {
    expect(normalizeServerConfig({ serverCount: 2, ports: [8080, 8081], onSlotsFull: 'ask' }).onSlotsFull).toBe('ask')
  })
})
