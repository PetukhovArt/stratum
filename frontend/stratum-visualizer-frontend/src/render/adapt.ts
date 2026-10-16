import {
  ContainerKind,
  EdgeKind,
  Severity,
  SnapshotContainer,
  SnapshotEdge,
  SnapshotLayer,
  SnapshotModule,
  SUPPORTED_SNAPSHOT_VERSION,
  Violation,
} from '../types'
import {
  ContainerKindLabel,
  DesignContainer,
  DesignData,
  DesignEdge,
  DesignLayer,
  DesignModule,
  DesignViolation,
  EdgeKindLabel,
  Facet,
  ModuleKind,
  SeverityLabel,
} from './design'
import { Model } from './model'

interface Source {
  modules: readonly SnapshotModule[]
  containers: readonly SnapshotContainer[]
  layers: readonly SnapshotLayer[]
  edges: readonly SnapshotEdge[]
}

const CYCLE_RULE = 'stratum/no-circular-deps'

const containerKey = (id: number): string => `c-${id}`
const containerModuleKey = (id: number): string => `cn-${id}`
const moduleKey = (id: number): string => `m-${id}`

export interface DesignRef {
  kind: 'module' | 'container'
  id: number
}

export const designKey = (ref: DesignRef): string =>
  ref.kind === 'module' ? moduleKey(ref.id) : containerModuleKey(ref.id)

const DESIGN_KEY = /^(m|cn)-(\d+)$/

export const parseDesignKey = (key: string): DesignRef | null => {
  const match = DESIGN_KEY.exec(key)
  if (!match) return null
  return { kind: match[1] === 'm' ? 'module' : 'container', id: Number(match[2]) }
}
const layerKey = (id: number): string => `l-${id}`
const violationKey = (idx: number): string => `v-${idx}`
const edgeKey = (from: number, to: number): string => `${from}->${to}`

const basename = (p: string): string => {
  const noExt = p.replace(/\.[^./\\]+$/, '')
  return noExt.split(/[/\\]/).pop() ?? noExt
}

const inferKind = (label: string): ModuleKind => {
  if (label === 'index') return 'composer'
  if (label.startsWith('_')) return 'shared'
  return 'regular'
}

const CONTAINER_KIND_LABEL: Record<ContainerKind, ContainerKindLabel> = {
  layer: 'layer',
  slice: 'slice',
  sub_unit: 'sub-unit',
  nested: 'nested',
}

const containerFacet = (c: SnapshotContainer): Facet => ({
  axis: 'stage',
  value: c.stage,
  kind: CONTAINER_KIND_LABEL[c.kind],
})

const edgeKindToLabel = (k: EdgeKind): EdgeKindLabel => {
  switch (k) {
    case EdgeKind.Di:
      return 'di'
    case EdgeKind.Runtime:
      return 'runtime'
    case EdgeKind.Static:
    default:
      return 'static'
  }
}

const severityToLabel = (s: Severity): SeverityLabel => {
  switch (s) {
    case Severity.Error:
      return 'error'
    case Severity.Warning:
      return 'warning'
    case Severity.Info:
    case Severity.Off:
    default:
      return 'info'
  }
}

const SEVERITY_RANK: Record<SeverityLabel, number> = { error: 3, warning: 2, info: 1 }

const upgradeSeverity = (current: SeverityLabel | null, next: SeverityLabel): SeverityLabel => {
  if (current === null) return next
  return SEVERITY_RANK[next] > SEVERITY_RANK[current] ? next : current
}

interface ModuleTree {
  modules: DesignModule[]
  byId: Map<string, DesignModule>
  childIndex: Record<string, string[]>
  topByContainer: Record<string, string[]>
  rootOf: (containerId: number) => number
}

const containerRootResolver = (
  snapContainers: ReadonlyMap<number, SnapshotContainer>,
): ((containerId: number) => number) => {
  const cache = new Map<number, number>()
  return (containerId) => {
    const cached = cache.get(containerId)
    if (cached !== undefined) return cached
    let cur = containerId
    const visited = new Set<number>()
    while (!visited.has(cur)) {
      visited.add(cur)
      const c = snapContainers.get(cur)
      if (!c || c.parent === null) break
      cur = c.parent
    }
    cache.set(containerId, cur)
    return cur
  }
}

const blankModule = (
  fields: Pick<DesignModule, 'id' | 'container' | 'parentId' | 'label' | 'path' | 'facet'>,
): DesignModule => ({
  ...fields,
  kind: inferKind(fields.label),
  depth: 0,
  hasChildren: false,
  violations: [],
  severity: null,
  descError: 0,
  descWarn: 0,
  descInfo: 0,
})

const buildModuleTree = (snapshot: Source): ModuleTree => {
  const snapContainers = new Map(snapshot.containers.map((c) => [c.id, c]))
  const rootOf = containerRootResolver(snapContainers)
  const modules: DesignModule[] = []
  const childIndex: Record<string, string[]> = {}
  const topByContainer: Record<string, string[]> = {}

  const attach = (mod: DesignModule): void => {
    modules.push(mod)
    if (mod.parentId) {
      ;(childIndex[mod.parentId] ??= []).push(mod.id)
    } else {
      ;(topByContainer[mod.container] ??= []).push(mod.id)
    }
  }

  for (const c of snapshot.containers) {
    if (c.parent === null) continue
    const parent = snapContainers.get(c.parent)
    const isParentRoot = parent !== undefined && parent.parent === null
    attach(
      blankModule({
        id: containerModuleKey(c.id),
        container: containerKey(rootOf(c.id)),
        parentId: isParentRoot ? null : containerModuleKey(c.parent),
        label: c.name,
        path: c.path,
        facet: containerFacet(c),
      }),
    )
  }

  for (const m of snapshot.modules) {
    const c = snapContainers.get(m.container)
    if (!c) continue
    attach(
      blankModule({
        id: moduleKey(m.id),
        container: containerKey(rootOf(m.container)),
        parentId: c.parent === null ? null : containerModuleKey(m.container),
        label: basename(m.path),
        path: m.path,
        facet: { axis: 'purity', value: m.purity },
      }),
    )
  }

  const byId = new Map(modules.map((m) => [m.id, m]))
  for (const m of modules) {
    m.hasChildren = (childIndex[m.id]?.length ?? 0) > 0
  }
  return { modules, byId, childIndex, topByContainer, rootOf }
}

const topIds = (tree: ModuleTree): string[] => Object.values(tree.topByContainer).flat()

const assignDepths = (tree: ModuleTree): void => {
  const visit = (id: string, depth: number): void => {
    const mod = tree.byId.get(id)
    if (!mod) return
    mod.depth = depth
    for (const kid of tree.childIndex[id] ?? []) visit(kid, depth + 1)
  }
  for (const id of topIds(tree)) visit(id, 0)
}

const adaptViolations = (violations: ReadonlyArray<Violation>): DesignViolation[] =>
  violations.map((v, idx) => {
    const ids = new Set<number>(v.modules)
    if (v.edge) {
      ids.add(v.edge.from)
      ids.add(v.edge.to)
    }
    return {
      id: violationKey(idx),
      rule: v.rule,
      severity: severityToLabel(v.severity),
      message: v.message,
      modules: [...ids].map(moduleKey),
      isCycle: v.rule === CYCLE_RULE,
    }
  })

const attachViolations = (tree: ModuleTree, violations: ReadonlyArray<DesignViolation>): void => {
  for (const v of violations) {
    for (const mid of v.modules) {
      const mod = tree.byId.get(mid)
      if (!mod) continue
      mod.violations.push(v.id)
      mod.severity = upgradeSeverity(mod.severity, v.severity)
    }
  }
}

const aggregateDescendantSeverity = (tree: ModuleTree): void => {
  const visit = (id: string): { err: number; warn: number; info: number } => {
    const totals = { err: 0, warn: 0, info: 0 }
    const m = tree.byId.get(id)
    if (!m) return totals
    if (m.severity === 'error') totals.err++
    if (m.severity === 'warning') totals.warn++
    if (m.severity === 'info') totals.info++
    for (const kid of tree.childIndex[id] ?? []) {
      const sub = visit(kid)
      totals.err += sub.err
      totals.warn += sub.warn
      totals.info += sub.info
    }
    m.descError = totals.err
    m.descWarn = totals.warn
    m.descInfo = totals.info
    return totals
  }
  for (const id of topIds(tree)) visit(id)
}

const buildEdges = (
  snapshot: Source,
  violations: ReadonlyArray<Violation>,
  designViolations: ReadonlyArray<DesignViolation>,
): DesignEdge[] => {
  const byEdge = new Map<string, DesignViolation>()
  violations.forEach((v, idx) => {
    if (!v.edge) return
    const key = edgeKey(v.edge.from, v.edge.to)
    const current = byEdge.get(key)
    const candidate = designViolations[idx]
    if (!current || SEVERITY_RANK[candidate.severity] > SEVERITY_RANK[current.severity]) {
      byEdge.set(key, candidate)
    }
  })
  return snapshot.edges.map((e) => {
    const v = byEdge.get(edgeKey(e.from, e.to)) ?? null
    return {
      source: moduleKey(e.from),
      target: moduleKey(e.to),
      kind: edgeKindToLabel(e.kind),
      violation: v?.id ?? null,
      severity: v?.severity ?? null,
    }
  })
}

const buildContainers = (snapshot: Source, tree: ModuleTree): DesignContainer[] => {
  const leafIdsByRoot = new Map<number, string[]>()
  for (const m of snapshot.modules) {
    const root = tree.rootOf(m.container)
    const ids = leafIdsByRoot.get(root) ?? []
    ids.push(moduleKey(m.id))
    leafIdsByRoot.set(root, ids)
  }
  return snapshot.containers
    .filter((c) => c.parent === null)
    .map((c) => {
      const id = containerKey(c.id)
      const leafIds = leafIdsByRoot.get(c.id) ?? []
      const severities = leafIds.map((mid) => tree.byId.get(mid)?.severity)
      return {
        id,
        layer: layerKey(c.layer),
        label: c.name,
        errors: severities.filter((s) => s === 'error').length,
        warnings: severities.filter((s) => s === 'warning').length,
        moduleCount: leafIds.length,
        stage: c.stage,
        kind: CONTAINER_KIND_LABEL[c.kind],
      }
    })
}

const buildLayers = (
  snapshot: Source,
  containers: DesignContainer[],
  hueOf: (index: number) => number,
): DesignLayer[] =>
  snapshot.layers.map((l, idx) => {
    const inLayer = containers.filter((c) => c.layer === layerKey(l.id))
    return {
      id: layerKey(l.id),
      label: l.name,
      order: idx,
      hue: hueOf(idx),
      modules: inLayer.reduce((a, c) => a + c.moduleCount, 0),
      errors: inLayer.reduce((a, c) => a + c.errors, 0),
      warnings: inLayer.reduce((a, c) => a + c.warnings, 0),
    }
  })

const indexLayersByContainer = (
  containers: DesignContainer[],
  layers: DesignLayer[],
): Map<string, DesignLayer> => {
  const layerById = new Map(layers.map((l) => [l.id, l]))
  const out = new Map<string, DesignLayer>()
  for (const c of containers) {
    const layer = layerById.get(c.layer)
    if (layer) {
      out.set(c.id, layer)
    }
  }
  return out
}

export const adaptModel = (model: Model): DesignData => {
  const source: Source = {
    modules: [...model.modules.values()],
    containers: [...model.containers.values()],
    layers: model.layers,
    edges: model.edges,
  }
  const tree = buildModuleTree(source)
  assignDepths(tree)
  const designViolations = adaptViolations(model.violations)
  attachViolations(tree, designViolations)
  aggregateDescendantSeverity(tree)
  const containers = buildContainers(source, tree)
  const layers = buildLayers(source, containers, model.layerHue)

  return {
    meta: {
      project: model.project,
      loadedAt: new Date().toISOString(),
      snapshotVersion: SUPPORTED_SNAPSHOT_VERSION,
    },
    layers,
    containers,
    modules: tree.modules,
    childIndex: tree.childIndex,
    topByContainer: tree.topByContainer,
    edges: buildEdges(source, model.violations, designViolations),
    violations: designViolations,
    moduleById: tree.byId,
    violationById: new Map(designViolations.map((v) => [v.id, v])),
    layerByContainer: indexLayersByContainer(containers, layers),
  }
}

