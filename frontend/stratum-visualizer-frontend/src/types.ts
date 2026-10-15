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
  purity: number
}

export const CONTAINER_KINDS = ['layer', 'slice', 'sub_unit', 'nested'] as const
export type ContainerKind = (typeof CONTAINER_KINDS)[number]

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
  rule: string
  severity: Severity
  message: string
  file: string
  location: { line: number; column: number }
  modules: number[]
  edge: ViolationEdge | null
  suggestion: string | null
}

export const SUPPORTED_SNAPSHOT_VERSION = 2
