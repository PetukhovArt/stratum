export interface Loc {
  /** Root-relative, `/`-separated. */
  path: string
  /** 1-based. */
  line: number
}

export interface ContractName {
  /** `*` for a namespace import or a re-export of the whole module; its `decl` is the module's line 1. */
  name: string
  isTypeOnly: boolean
  /** `null` when the name did not resolve to a declaration. */
  decl: Loc | null
}

export interface EdgeImport {
  file: string
  line: number
  isTypeOnly: boolean
  kind: string
  names: ContractName[]
}

export type EdgeEnds = { level: 'module' | 'container'; from: number; to: number }

export interface SourceText {
  path: string
  /** 1-based line number of `lines[0]`. */
  start: number
  lines: string[]
  total: number
}

export class ApiStatusError extends Error {
  constructor(
    readonly url: string,
    readonly status: number,
  ) {
    super(`${url} answered ${status}`)
    this.name = 'ApiStatusError'
  }
}

export class ApiShapeError extends Error {
  constructor(url: string, where: string) {
    super(`Malformed answer of ${url}: ${where}`)
    this.name = 'ApiShapeError'
  }
}

type Fields = Record<string, unknown>

const isFields = (value: unknown): value is Fields => typeof value === 'object' && value !== null

const isLine = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 1

const parseLoc = (raw: unknown): Loc | null => {
  if (!isFields(raw) || typeof raw.path !== 'string' || !isLine(raw.line)) return null
  return { path: raw.path, line: raw.line }
}

const parseName = (raw: unknown, url: string, where: string): ContractName => {
  if (!isFields(raw) || typeof raw.name !== 'string' || typeof raw.type_only !== 'boolean') {
    throw new ApiShapeError(url, where)
  }
  const decl = parseLoc(raw.decl)
  if (raw.decl !== null && raw.decl !== undefined && !decl) throw new ApiShapeError(url, `${where}.decl`)
  return { name: raw.name, isTypeOnly: raw.type_only, decl }
}

const parseEdge = (raw: unknown, url: string): EdgeImport[] => {
  if (!isFields(raw) || !Array.isArray(raw.imports)) throw new ApiShapeError(url, '`imports` is not an array')
  return raw.imports.map((item: unknown, i) => {
    const where = `imports[${i}]`
    if (
      !isFields(item) ||
      typeof item.file !== 'string' ||
      !isLine(item.line) ||
      typeof item.type_only !== 'boolean' ||
      typeof item.kind !== 'string' ||
      !Array.isArray(item.names)
    ) {
      throw new ApiShapeError(url, where)
    }
    return {
      file: item.file,
      line: item.line,
      isTypeOnly: item.type_only,
      kind: item.kind,
      names: item.names.map((name: unknown, j) => parseName(name, url, `${where}.names[${j}]`)),
    }
  })
}

const parseSource = (raw: unknown, url: string): SourceText => {
  if (
    !isFields(raw) ||
    typeof raw.path !== 'string' ||
    !isLine(raw.start) ||
    !Array.isArray(raw.lines) ||
    !raw.lines.every((line: unknown) => typeof line === 'string') ||
    typeof raw.total !== 'number'
  ) {
    throw new ApiShapeError(url, 'not { path, start, lines, total }')
  }
  return { path: raw.path, start: raw.start, lines: raw.lines, total: raw.total }
}

const cachedJson = <K, T>(urlOf: (key: K) => string, parse: (raw: unknown, url: string) => T) => {
  const cache = new Map<string, Promise<T>>()
  const load = async (url: string): Promise<T> => {
    const resp = await fetch(url)
    if (!resp.ok) throw new ApiStatusError(url, resp.status)
    return parse(await resp.json(), url)
  }
  return (key: K): Promise<T> => {
    const url = urlOf(key)
    const hit = cache.get(url)
    if (hit) return hit
    const pending = load(url)
    cache.set(url, pending)
    pending.catch(() => cache.delete(url))
    return pending
  }
}

const edgeUrl = (ends: EdgeEnds): string =>
  ends.level === 'module'
    ? `/api/edge?from=${ends.from}&to=${ends.to}`
    : `/api/edge?from_container=${ends.from}&to_container=${ends.to}`

/** Cached per edge; a rejected request is dropped from the cache so the next call retries. */
export const fetchEdge = cachedJson(edgeUrl, parseEdge)

/**
 * `context` lines on each side of `line`, cached like `fetchEdge`.
 * Rejects with `ApiStatusError` 404 for a missing file and 400 for a path outside ROOT.
 */
export const fetchSource = cachedJson(
  (at: Loc & { context: number }) =>
    `/api/source?path=${encodeURIComponent(at.path)}&line=${at.line}&context=${at.context}`,
  parseSource,
)

export interface Feature {
  id: string
  name: string
  /** Snapshot module ids. */
  entry: number[]
  /** Snapshot module ids: the entries and everything they reach by static imports. */
  modules: number[]
}

export interface FeatureIndex {
  features: Feature[]
  /** Feature ids chosen on the command line (`visualize --feature ID`). */
  selected: string[]
}

const isIds = (value: unknown): value is number[] =>
  Array.isArray(value) && value.every((id: unknown) => Number.isInteger(id))

const isStrings = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((id: unknown) => typeof id === 'string')

const parseFeatures = (raw: unknown, url: string): FeatureIndex => {
  if (!isFields(raw) || !Array.isArray(raw.features) || !isStrings(raw.selected)) {
    throw new ApiShapeError(url, 'not { features, shared, selected }')
  }
  const features = raw.features.map((item: unknown, i): Feature => {
    if (
      !isFields(item) ||
      typeof item.id !== 'string' ||
      typeof item.name !== 'string' ||
      !isIds(item.entry) ||
      !isIds(item.modules)
    ) {
      throw new ApiShapeError(url, `features[${i}]`)
    }
    return { id: item.id, name: item.name, entry: item.entry, modules: item.modules }
  })
  return { features, selected: raw.selected }
}

/** Cached like `fetchEdge`; rejects with `ApiStatusError` 404 when the server has no features. */
export const fetchFeatures = cachedJson((_whole: void) => '/api/features', parseFeatures)

export interface SystemInfo {
  id: string
  name: string
  contractsTotal: number
  unavailableReason: string | null
}

export interface ContractCaller {
  /** Snapshot module id. */
  module: number
  line: number
  owner: string | null
}

export interface SystemContract {
  id: string
  signature: string | null
  /** At a `system-id:rel/path` that `fetchSource` serves as is; `null` when no declaration matched the call. */
  decl: Loc | null
  callers: ContractCaller[]
}

export interface Channel {
  from: string
  to: string
  contracts: SystemContract[]
  unattributedCalls: number
}

export interface SystemIndex {
  self: SystemInfo
  others: SystemInfo[]
  channels: Channel[]
}

type ParsedSystem = SystemInfo & { isSelf: boolean }

const isCount = (value: unknown): value is number => Number.isInteger(value) && (value as number) >= 0

const parseSystem = (raw: unknown, url: string, where: string): ParsedSystem => {
  if (
    !isFields(raw) ||
    typeof raw.id !== 'string' ||
    typeof raw.name !== 'string' ||
    typeof raw.self !== 'boolean' ||
    !isCount(raw.contracts_total) ||
    (raw.unavailable !== undefined && typeof raw.unavailable !== 'string')
  ) {
    throw new ApiShapeError(url, where)
  }
  return {
    id: raw.id,
    name: raw.name,
    isSelf: raw.self,
    contractsTotal: raw.contracts_total,
    unavailableReason: raw.unavailable ?? null,
  }
}

const parseCaller = (raw: unknown, url: string, where: string): ContractCaller => {
  if (!isFields(raw) || !isCount(raw.module) || !isLine(raw.line) || (raw.owner !== null && typeof raw.owner !== 'string')) {
    throw new ApiShapeError(url, where)
  }
  return { module: raw.module, line: raw.line, owner: raw.owner }
}

const parseSystemContract = (raw: unknown, url: string, where: string): SystemContract => {
  if (
    !isFields(raw) ||
    typeof raw.id !== 'string' ||
    (raw.signature !== null && typeof raw.signature !== 'string') ||
    !Array.isArray(raw.callers)
  ) {
    throw new ApiShapeError(url, where)
  }
  const decl = parseLoc(raw.decl)
  if (raw.decl !== null && !decl) throw new ApiShapeError(url, `${where}.decl`)
  return {
    id: raw.id,
    signature: raw.signature,
    decl,
    callers: raw.callers.map((c: unknown, i) => parseCaller(c, url, `${where}.callers[${i}]`)),
  }
}

const parseChannel = (raw: unknown, url: string, where: string): Channel => {
  if (
    !isFields(raw) ||
    typeof raw.from !== 'string' ||
    typeof raw.to !== 'string' ||
    !Array.isArray(raw.contracts) ||
    !isCount(raw.unattributed)
  ) {
    throw new ApiShapeError(url, where)
  }
  return {
    from: raw.from,
    to: raw.to,
    contracts: raw.contracts.map((c: unknown, i) => parseSystemContract(c, url, `${where}.contracts[${i}]`)),
    unattributedCalls: raw.unattributed,
  }
}

const infoOf = ({ isSelf: _isSelf, ...system }: ParsedSystem): SystemInfo => system

const parseSystems = (raw: unknown, url: string): SystemIndex => {
  if (!isFields(raw) || !Array.isArray(raw.systems) || !Array.isArray(raw.channels)) {
    throw new ApiShapeError(url, 'not { systems, channels }')
  }
  const systems = raw.systems.map((s: unknown, i) => parseSystem(s, url, `systems[${i}]`))
  const selves = systems.filter((s) => s.isSelf)
  if (selves.length !== 1) throw new ApiShapeError(url, `${selves.length} systems are self, expected 1`)
  return {
    self: infoOf(selves[0]),
    others: systems.filter((s) => !s.isSelf).map(infoOf),
    channels: raw.channels.map((c: unknown, i) => parseChannel(c, url, `channels[${i}]`)),
  }
}

/** Cached like `fetchEdge`; rejects with `ApiStatusError` 404 when the config declares no `systems`. */
export const fetchSystems = cachedJson((_whole: void) => '/api/systems', parseSystems)
