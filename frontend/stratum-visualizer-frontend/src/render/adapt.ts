import { EdgeKind, GraphSnapshot, Severity, Violation } from '../types'
import {
  DesignContainer,
  DesignData,
  DesignEdge,
  DesignLayer,
  DesignModule,
  DesignViolation,
  EdgeKindLabel,
  ModuleKind,
  SeverityLabel,
} from './design'

// Layer hue palette — cycled by layer index so renderer stays deterministic.
const LAYER_HUE_PALETTE = [28, 56, 145, 220, 290, 0, 100, 180, 260, 340]
const layerHue = (idx: number): number => LAYER_HUE_PALETTE[idx % LAYER_HUE_PALETTE.length]

const containerKey = (id: number): string => `c-${id}`
const containerModuleKey = (id: number): string => `cn-${id}`
const moduleKey = (id: number): string => `m-${id}`

const basename = (p: string): string => {
  const noExt = p.replace(/\.[^./\\]+$/, '')
  return noExt.split(/[/\\]/).pop() ?? noExt
}

const inferKind = (label: string): ModuleKind => {
  if (label === 'index') return 'composer'
  if (label.startsWith('_')) return 'shared'
  return 'regular'
}

const SEGMENT_LIKE: ReadonlySet<string> = new Set(['model', 'api', 'ui', 'lib', 'config', 'hooks'])

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

const upgradeSeverity = (current: SeverityLabel | null, next: SeverityLabel): SeverityLabel => {
  const rank = (s: SeverityLabel | null): number =>
    s === 'error' ? 3 : s === 'warning' ? 2 : s === 'info' ? 1 : 0
  return rank(next) > rank(current) ? next : (current ?? next)
}

export interface AdaptOptions {
  project?: string
}

export const adaptSnapshot = (
  snapshot: GraphSnapshot,
  violations: ReadonlyArray<Violation> = [],
  options: AdaptOptions = {},
): DesignData => {
  const snapContainers = new Map(snapshot.containers.map((c) => [c.id, c]))

  const rootCache = new Map<number, number>()
  const findRoot = (cid: number): number => {
    const cached = rootCache.get(cid)
    if (cached !== undefined) return cached
    let cur = cid
    const visited = new Set<number>()
    while (true) {
      if (visited.has(cur)) break
      visited.add(cur)
      const c = snapContainers.get(cur)
      if (!c || c.parent === null) break
      cur = c.parent
    }
    rootCache.set(cid, cur)
    return cur
  }
  for (const c of snapshot.containers) findRoot(c.id)

  // ── 1. Emit modules (nested containers → compound modules; files → leaves) ──
  const modules: DesignModule[] = []
  const childIndex: Record<string, string[]> = {}
  const topByContainer: Record<string, string[]> = {}

  const pushAsChild = (parentId: string | null, rootContainer: string, id: string): void => {
    if (parentId) {
      ;(childIndex[parentId] ??= []).push(id)
    } else {
      ;(topByContainer[rootContainer] ??= []).push(id)
    }
  }

  for (const c of snapshot.containers) {
    if (c.parent === null) continue
    const rootId = findRoot(c.id)
    const rootKey = containerKey(rootId)
    const id = containerModuleKey(c.id)
    const parentSnap = snapContainers.get(c.parent)
    const parentId = parentSnap && parentSnap.parent === null ? null : containerModuleKey(c.parent)
    modules.push({
      id,
      container: rootKey,
      parentId,
      label: c.name,
      path: c.name,
      loc: 0,
      stage: 1,
      kind: inferKind(c.name),
      depth: 0,
      hasChildren: true,
      violations: [],
      severity: null,
      descError: 0,
      descWarn: 0,
      descInfo: 0,
    })
    pushAsChild(parentId, rootKey, id)
  }

  for (const m of snapshot.modules) {
    const c = snapContainers.get(m.container)
    if (!c) continue
    const rootId = findRoot(m.container)
    const rootKey = containerKey(rootId)
    const parentId = c.parent === null ? null : containerModuleKey(m.container)
    const label = basename(m.path)
    const id = moduleKey(m.id)
    modules.push({
      id,
      container: rootKey,
      parentId,
      label,
      path: m.path,
      loc: 0,
      stage: m.stage || 1,
      kind: inferKind(label),
      depth: 0,
      hasChildren: false,
      violations: [],
      severity: null,
      descError: 0,
      descWarn: 0,
      descInfo: 0,
    })
    pushAsChild(parentId, rootKey, id)
  }

  const byId = new Map(modules.map((m) => [m.id, m]))
  for (const m of modules) m.hasChildren = (childIndex[m.id]?.length ?? 0) > 0

  const setDepth = (id: string, d: number): void => {
    const mod = byId.get(id)
    if (!mod) return
    mod.depth = d
    for (const kid of childIndex[id] ?? []) setDepth(kid, d + 1)
  }
  for (const cid of Object.keys(topByContainer)) {
    for (const tid of topByContainer[cid]) setDepth(tid, 0)
  }

  // ── 2. Violations + attach to modules ──
  const designViolations: DesignViolation[] = violations.map((v, idx) => ({
    id: `v-${idx}`,
    rule: ruleLabel(v.rule),
    severity: severityToLabel(v.severity),
    message: v.message,
    ruleDoc: `stratum.dev/rules/${ruleSlug(v.rule)}`,
  }))

  violations.forEach((v, idx) => {
    const vid = `v-${idx}`
    const sevLabel = severityToLabel(v.severity)
    const targets = new Set<number>(v.modules)
    if (v.edge) {
      targets.add(v.edge.from)
      targets.add(v.edge.to)
    }
    for (const mid of targets) {
      const dm = byId.get(moduleKey(mid))
      if (!dm) continue
      if (!dm.violations.includes(vid)) dm.violations.push(vid)
      dm.severity = upgradeSeverity(dm.severity, sevLabel)
    }
  })

  const aggregate = (id: string): { err: number; warn: number; info: number } => {
    const m = byId.get(id)
    if (!m) return { err: 0, warn: 0, info: 0 }
    const r = { err: 0, warn: 0, info: 0 }
    if (m.severity === 'error') r.err++
    else if (m.severity === 'warning') r.warn++
    else if (m.severity === 'info') r.info++
    for (const cid of childIndex[id] ?? []) {
      const sub = aggregate(cid)
      r.err += sub.err
      r.warn += sub.warn
      r.info += sub.info
    }
    m.descError = r.err
    m.descWarn = r.warn
    m.descInfo = r.info
    return r
  }
  for (const cid of Object.keys(topByContainer)) {
    for (const tid of topByContainer[cid]) aggregate(tid)
  }

  // ── 3. Stage inference for compound modules ──
  const computeStage = (id: string): number => {
    const m = byId.get(id)
    if (!m) return 1
    const kidIds = childIndex[id] ?? []
    if (kidIds.length === 0) return m.stage || 1
    const kids = kidIds.map((k) => byId.get(k)).filter((k): k is DesignModule => !!k)
    const hasGrandkids = kids.some((k) => (childIndex[k.id]?.length ?? 0) > 0)
    if (!hasGrandkids) return 2
    const hasComposer = kids.some((k) => k.kind === 'composer')
    const subFeatures = kids.filter(
      (k) =>
        (childIndex[k.id]?.length ?? 0) > 0 &&
        !SEGMENT_LIKE.has(k.label) &&
        k.kind === 'regular',
    )
    if (hasComposer && subFeatures.length >= 2) return 4
    return 3
  }
  for (const m of modules) if (m.hasChildren) m.stage = computeStage(m.id)

  // ── 4. Edges ──
  const edgeViolation = (from: number, to: number): string | null => {
    for (let i = 0; i < violations.length; i++) {
      const v = violations[i]
      if (v.edge && v.edge.from === from && v.edge.to === to) return `v-${i}`
    }
    return null
  }
  const edges: DesignEdge[] = snapshot.edges.map((e) => ({
    source: moduleKey(e.from),
    target: moduleKey(e.to),
    kind: edgeKindToLabel(e.kind),
    violation: edgeViolation(e.from, e.to),
  }))

  // ── 5. Layers + containers ──
  const containerStage = (rootCid: number): number => {
    const tops = (topByContainer[containerKey(rootCid)] ?? [])
      .map((tid) => byId.get(tid))
      .filter((m): m is DesignModule => !!m)
    if (tops.length === 0) return 1
    if (tops.length === 1 && !tops[0].hasChildren) return 1
    const hasComposer = tops.some((t) => t.kind === 'composer')
    const segmentLike = tops.filter((t) => SEGMENT_LIKE.has(t.label))
    const subFeatures = tops.filter(
      (t) => t.hasChildren && !SEGMENT_LIKE.has(t.label) && t.kind === 'regular',
    )
    if (hasComposer && subFeatures.length >= 2) return 4
    if (segmentLike.length >= 2) return 3
    if (tops.length === 1) return 1
    return 2
  }

  const containers: DesignContainer[] = snapshot.containers
    .filter((c) => c.parent === null)
    .map((c) => {
      const moduleIdsInRoot = snapshot.modules
        .filter((m) => findRoot(m.container) === c.id)
        .map((m) => moduleKey(m.id))
      let errors = 0
      let warnings = 0
      for (const mid of moduleIdsInRoot) {
        const dm = byId.get(mid)
        if (dm?.severity === 'error') errors++
        else if (dm?.severity === 'warning') warnings++
      }
      return {
        id: containerKey(c.id),
        layer: `l-${c.layer}`,
        label: c.name,
        errors,
        warnings,
        moduleCount: moduleIdsInRoot.length,
        stage: containerStage(c.id),
      }
    })

  const layers: DesignLayer[] = snapshot.layers.map((l, idx) => {
    const inLayer = containers.filter((c) => c.layer === `l-${l.id}`)
    return {
      id: `l-${l.id}`,
      label: l.name,
      order: idx,
      hue: layerHue(idx),
      desc: '',
      modules: inLayer.reduce((a, c) => a + c.moduleCount, 0),
      errors: inLayer.reduce((a, c) => a + c.errors, 0),
      warnings: inLayer.reduce((a, c) => a + c.warnings, 0),
    }
  })

  return {
    meta: {
      project: options.project ?? 'project',
      generatedAt: new Date().toISOString(),
      snapshotVersion: snapshot.version,
    },
    layers,
    containers,
    modules,
    childIndex,
    topByContainer,
    edges,
    violations: designViolations,
  }
}

// TODO: when the rule registry is exposed via /api, replace these with real labels.
const ruleLabel = (id: number): string => `rule-${id}`
const ruleSlug = (id: number): string => `rule-${id}`
