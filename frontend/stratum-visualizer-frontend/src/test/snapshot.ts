import { GraphSnapshot, SnapshotModule, Territory } from '../types'

export interface SnapshotInput {
  modules: Omit<SnapshotModule, 'territory'>[]
  containers: GraphSnapshot['containers']
  layers: GraphSnapshot['layers']
  edges: GraphSnapshot['edges']
}

const OPEN_TERRITORY: Territory = {
  may_import_layers: [],
  imported_by_layers: [],
  max_import_purity: 4,
  visible_zones: [],
  may_not: [],
}

export const snapshotV3 = (input: SnapshotInput): GraphSnapshot => ({
  version: 3,
  project: 'test',
  zones: [],
  territories: [OPEN_TERRITORY],
  ...input,
  modules: input.modules.map((m) => ({ territory: 0, ...m })),
})
