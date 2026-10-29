import { Violation } from '../types'
import { parseSnapshot } from './snapshot'

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

/** Set from `window.__STRATUM_STATIC__`, which `stratum-lint export` writes into its `index.html`; `null` under `visualize`. */
export interface StaticSite {
  hasSource: boolean
  hasSystems: boolean
  hasDrift: boolean
}

declare global {
  interface Window {
    __STRATUM_STATIC__?: unknown
  }
}

const DATA = './data'

export const staticSite = (): StaticSite | null => {
  const flags: unknown = typeof window === 'undefined' ? undefined : window.__STRATUM_STATIC__
  if (!isFields(flags)) return null
  return { hasSource: flags.source === true, hasSystems: flags.systems === true, hasDrift: flags.drift === true }
}

export class SourceNotExportedError extends Error {
  constructor(readonly path: string) {
    super(`${path}: the site was exported without --with-source`)
    this.name = 'SourceNotExportedError'
  }
}

const getJson = async (url: string): Promise<unknown> => {
  const resp = await fetch(url)
  if (!resp.ok) throw new ApiStatusError(url, resp.status)
  return resp.json()
}

const cached = <K, T>(keyOf: (key: K) => string, load: (key: K, url: string) => Promise<T>) => {
  const cache = new Map<string, Promise<T>>()
  return (key: K): Promise<T> => {
    const url = keyOf(key)
    const hit = cache.get(url)
    if (hit) return hit
    const pending = load(key, url)
    cache.set(url, pending)
    pending.catch(() => cache.delete(url))
    return pending
  }
}

const cachedJson = <K, T>(urlOf: (key: K) => string, parse: (raw: unknown, url: string) => T) =>
  cached(urlOf, async (_key: K, url) => parse(await getJson(url), url))

type Whole = 'snapshot' | 'violations' | 'features' | 'systems' | 'drift'

const isExported = (whole: Whole, site: StaticSite): boolean =>
  (whole !== 'systems' || site.hasSystems) && (whole !== 'drift' || site.hasDrift)

const wholeJson = <T>(whole: Whole, parse: (raw: unknown, url: string) => T) =>
  cached(
    (_whole: void) => (staticSite() ? `${DATA}/${whole}.json` : `/api/${whole}`),
    async (_whole: void, url) => {
      const site = staticSite()
      if (site && !isExported(whole, site)) throw new ApiStatusError(url, 404)
      return parse(await getJson(url), url)
    },
  )

type EdgeFile = Record<string, Fields>
type MemberFile = Record<string, number[]>
type SourceIndex = Record<string, number>

const isEdgeFile = (raw: unknown): raw is EdgeFile => isFields(raw) && Object.values(raw).every(isFields)

const isMemberFile = (raw: unknown): raw is MemberFile => isFields(raw) && Object.values(raw).every(isIds)

const isSourceIndex = (raw: unknown): raw is SourceIndex => isFields(raw) && Object.values(raw).every(isCount)

const staticEdges = cachedJson(
  (_whole: void) => `${DATA}/edges.json`,
  (raw, url): EdgeFile => {
    if (!isEdgeFile(raw)) throw new ApiShapeError(url, 'not { from: { to: edge } }')
    return raw
  },
)

const staticMembers = cachedJson(
  (_whole: void) => `${DATA}/members.json`,
  (raw, url): MemberFile => {
    if (!isMemberFile(raw)) throw new ApiShapeError(url, 'not { container: [module] }')
    return raw
  },
)

const byFileThenLine = (a: EdgeImport, b: EdgeImport): number =>
  a.file === b.file ? a.line - b.line : a.file < b.file ? -1 : 1

const staticEdge = async (ends: EdgeEnds): Promise<EdgeImport[]> => {
  const url = `${DATA}/edges.json`
  const edges = await staticEdges()
  if (ends.level === 'module') return parseEdge(edges[ends.from]?.[ends.to] ?? { imports: [] }, url)
  const members = await staticMembers()
  const targets = new Set(members[ends.to] ?? [])
  return (members[ends.from] ?? [])
    .flatMap((from) => Object.entries(edges[from] ?? {}).filter(([to]) => targets.has(Number(to))))
    .flatMap(([, edge]) => parseEdge(edge, url))
    .sort(byFileThenLine)
}

const edgeUrl = (ends: EdgeEnds): string =>
  ends.level === 'module'
    ? `/api/edge?from=${ends.from}&to=${ends.to}`
    : `/api/edge?from_container=${ends.from}&to_container=${ends.to}`

/** Cached per edge; a rejected request is dropped from the cache so the next call retries. */
export const fetchEdge = cached(edgeUrl, async (ends: EdgeEnds, url) =>
  staticSite() ? staticEdge(ends) : parseEdge(await getJson(url), url),
)

type SourceWindow = Loc & { context: number }

const staticSourceIndex = cachedJson(
  (_whole: void) => `${DATA}/source/index.json`,
  (raw, url): SourceIndex => {
    if (!isSourceIndex(raw)) throw new ApiShapeError(url, 'not { path: n }')
    return raw
  },
)

const staticSourceFile = cachedJson(
  (n: number) => `${DATA}/source/${n}.json`,
  (raw, url): { path: string; lines: string[] } => {
    if (!isFields(raw) || typeof raw.path !== 'string' || !isStrings(raw.lines)) {
      throw new ApiShapeError(url, 'not { path, lines }')
    }
    return { path: raw.path, lines: raw.lines }
  },
)

const staticSource = async (at: SourceWindow, site: StaticSite): Promise<SourceText> => {
  if (!site.hasSource) throw new SourceNotExportedError(at.path)
  const n = (await staticSourceIndex())[at.path]
  if (n === undefined) throw new ApiStatusError(`${DATA}/source/index.json#${at.path}`, 404)
  const file = await staticSourceFile(n)
  const start = Math.max(1, at.line - at.context)
  return { path: file.path, start, lines: file.lines.slice(start - 1, at.line + at.context), total: file.lines.length }
}

/**
 * `context` lines on each side of `line`, cached like `fetchEdge`.
 * Rejects with `ApiStatusError` 404 for a missing file and 400 for a path outside ROOT,
 * and with `SourceNotExportedError` on a static site exported without `--with-source`.
 */
export const fetchSource = cached(
  (at: SourceWindow) => `/api/source?path=${encodeURIComponent(at.path)}&line=${at.line}&context=${at.context}`,
  async (at: SourceWindow, url) => {
    const site = staticSite()
    return site ? staticSource(at, site) : parseSource(await getJson(url), url)
  },
)

/** `null` when neither the server nor the static site has a brief for the module. */
export const fetchBrief = async (module: { id: number; path: string }): Promise<string | null> => {
  const url = staticSite()
    ? `${DATA}/context/${module.id}.txt`
    : `/api/context?path=${encodeURIComponent(module.path)}`
  const resp = await fetch(url)
  return resp.ok ? resp.text() : null
}

export const fetchSnapshot = wholeJson('snapshot', (raw) => parseSnapshot(raw))

export const fetchViolations = wholeJson('violations', (raw, url): Violation[] => {
  if (!Array.isArray(raw)) throw new ApiShapeError(url, 'not an array')
  return raw
})

/** Rejects with `ApiStatusError` 404 without `--base`; the answer is unparsed. */
export const fetchDrift = wholeJson('drift', (raw): unknown => raw)

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
export const fetchFeatures = wholeJson('features', parseFeatures)

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
  /** Ids of the features that reach this call by name; `null` from a server that does not send them. */
  features: readonly string[] | null
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
  if (
    !isFields(raw) ||
    !isCount(raw.module) ||
    !isLine(raw.line) ||
    (raw.owner !== null && typeof raw.owner !== 'string') ||
    (raw.features !== undefined && !isStrings(raw.features))
  ) {
    throw new ApiShapeError(url, where)
  }
  return { module: raw.module, line: raw.line, owner: raw.owner, features: raw.features ?? null }
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
export const fetchSystems = wholeJson('systems', parseSystems)
