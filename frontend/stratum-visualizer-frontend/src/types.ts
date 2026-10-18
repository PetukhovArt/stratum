export const enum EdgeKind {
  Static = 'static',
  Di = 'di',
  Runtime = 'runtime',
}

export const enum Severity {
  Off = 'off',
  Info = 'info',
  Warning = 'warning',
  Error = 'error',
}

export const SUPPORTED_SNAPSHOT_VERSION = 3

export const CROSS_LAYER_RULE = 'stratum/no-cross-layer-import'

export const CONTAINER_KINDS = ['layer', 'slice', 'sub_unit', 'nested'] as const
export type ContainerKind = (typeof CONTAINER_KINDS)[number]

export interface SnapshotModule {
  id: number
  /** Root-relative, `/`-separated. */
  path: string
  container: number
  layer: number
  purity: number
  /** Index into `GraphSnapshot.zones`. */
  zone?: number | null
  /** Index into `GraphSnapshot.territories`. */
  territory: number
}

export interface SnapshotContainer {
  id: number
  name: string
  path: string
  kind: ContainerKind
  layer: number
  parent: number | null
  stage: number
  entry: number | null
}

export interface SnapshotLayer {
  id: number
  name: string
  depends_on: number[]
}

export interface SnapshotEdge {
  from: number
  to: number
  kind: EdgeKind
}

export interface SnapshotZone {
  id: number
  path: string
  /** The folder whose modules may see into the zone. */
  root: string
}

export type Region =
  | { kind: 'siblings'; parent: number; containerKind: ContainerKind; containers: number[] }
  | { kind: 'internals' }

export interface Forbidden {
  rule: string
  region: Region
}

export interface Territory {
  may_import_layers: number[]
  imported_by_layers: number[]
  max_import_purity: number
  visible_zones: number[]
  may_not: Forbidden[]
}

export interface GraphSnapshot {
  version: typeof SUPPORTED_SNAPSHOT_VERSION
  project: string
  modules: SnapshotModule[]
  containers: SnapshotContainer[]
  layers: SnapshotLayer[]
  edges: SnapshotEdge[]
  zones: SnapshotZone[]
  territories: Territory[]
}

export interface ViolationEdge {
  from: number
  to: number
  kind: EdgeKind
}

export interface Violation {
  rule: string
  severity: Severity
  message: string
  file: string
  location: { line: number; column: number }
  modules: number[]
  edge: ViolationEdge | null
  fix: string | null
}
