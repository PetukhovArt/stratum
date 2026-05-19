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

export interface SnapshotModule {
  id: number
  path: string
  container: number
  layer: number
  stage: number
}

export interface SnapshotContainer {
  id: number
  name: string
  layer: number
  parent: number | null
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

export interface GraphSnapshot {
  version: number
  modules: SnapshotModule[]
  containers: SnapshotContainer[]
  layers: SnapshotLayer[]
  edges: SnapshotEdge[]
}

export interface ViolationEdge {
  from: number
  to: number
  kind: EdgeKind
}

export interface Violation {
  rule: number
  severity: Severity
  message: string
  file: string
  location: { line: number; column: number }
  modules: number[]
  edge: ViolationEdge | null
  suggestion: string | null
}

export const SUPPORTED_SNAPSHOT_VERSION = 1
