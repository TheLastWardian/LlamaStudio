export type SlotsFullMode = 'ask' | 'replace_first'

export interface ServerConfigRaw {
  port?: number
  serverCount?: number
  ports?: number[]
  chatPort?: number
  onSlotsFull?: SlotsFullMode
}

export interface ServerConfigNormalized {
  serverCount: number
  ports: number[]
  chatPort: number
  onSlotsFull: SlotsFullMode
}

function validPort(p: unknown): p is number {
  return typeof p === 'number' && Number.isFinite(p) && p >= 1 && p <= 65535
}

export function normalizeServerConfig(raw: ServerConfigRaw): ServerConfigNormalized {
  let serverCount = raw.serverCount ?? 1
  if (!Number.isFinite(serverCount) || serverCount < 1) serverCount = 1
  if (serverCount > 10) serverCount = 10

  let ports: number[]
  if (raw.ports && raw.ports.length > 0) {
    ports = raw.ports.map(p => (validPort(p) ? Math.round(p) : 8080))
  } else {
    ports = [validPort(raw.port) ? Math.round(raw.port) : 8080]
  }
  while (ports.length < serverCount) ports.push(ports[ports.length - 1] + 1)
  ports = ports.slice(0, serverCount)

  const chatPort = validPort(raw.chatPort) && ports.includes(raw.chatPort) ? raw.chatPort : ports[0]

  return {
    serverCount,
    ports,
    chatPort,
    onSlotsFull: raw.onSlotsFull === 'ask' ? 'ask' : 'replace_first',
  }
}
