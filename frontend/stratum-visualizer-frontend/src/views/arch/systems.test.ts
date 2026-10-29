import { describe, expect, it } from 'vitest'
import { SystemContract, SystemIndex } from '../../lib/api'
import { buildModel } from '../../render/model'
import { snapshotV3 } from '../../test/snapshot'
import { systemsOf } from './systems'

const container = (id: number, name: string, path: string, kind: 'layer' | 'slice', layer: number, parent: number | null) => ({
  id,
  name,
  path,
  kind,
  layer,
  parent,
  stage: 2,
  entry: null,
})

const SNAPSHOT = snapshotV3({
  layers: [
    { id: 0, name: 'features', depends_on: [1] },
    { id: 1, name: 'shared', depends_on: [] },
  ],
  containers: [
    container(0, 'features', 'src/features', 'layer', 0, null),
    container(1, 'shared', 'src/shared', 'layer', 1, null),
    container(2, 'checkout', 'src/features/checkout', 'slice', 0, 0),
    container(3, 'files', 'src/features/files', 'slice', 0, 0),
  ],
  modules: [
    { id: 0, path: 'src/features/checkout/index.ts', container: 2, layer: 0, purity: 2 },
    { id: 1, path: 'src/features/files/index.ts', container: 3, layer: 0, purity: 2 },
    { id: 2, path: 'src/shared/api/orders.ts', container: 1, layer: 1, purity: 2 },
    { id: 3, path: 'src/shared/api/users.ts', container: 1, layer: 1, purity: 2 },
  ],
  edges: [],
})

const contract = (id: string, callers: [module: number, line: number][]): SystemContract => ({
  id,
  signature: null,
  decl: null,
  callers: callers.map(([module, line]) => ({ module, line, owner: null })),
})

const system = (id: string) => ({ id, name: id, contractsTotal: 2, unavailableReason: null })

const INDEX: SystemIndex = {
  self: { ...system('ui'), contractsTotal: 0 },
  others: [system('host'), system('api'), { ...system('billing'), unavailableReason: 'no root' }],
  channels: [
    {
      from: 'ui',
      to: 'host',
      contracts: [contract('greet', [[1, 7]]), contract('save_file', [[1, 4], [0, 9]])],
      unattributedCalls: 1,
    },
    {
      from: 'ui',
      to: 'api',
      contracts: [contract('UserService.GetUser', [[3, 7]]), contract('UserService.WatchUsers', [[3, 10]])],
      unattributedCalls: 0,
    },
  ],
}

const only = (ids: number[]) => buildModel(SNAPSHOT, [], { keeps: (id) => ids.includes(id), isShared: () => false })

const inUse = (ids: number[] | null) => {
  const s = systemsOf(INDEX, ids ? only(ids) : buildModel(SNAPSHOT, []))
  return s.channels.map((c) => [c.to.id, c.contracts.map((x) => [x.id, x.callers.map((caller) => caller.module)])])
}

describe('systems in the model', () => {
  it('keeps a contract only while one of its callers is in the filtered model, and only those callers', () => {
    expect(inUse(null)).toEqual([
      ['host', [['greet', [1]], ['save_file', [1, 0]]]],
      ['api', [['UserService.GetUser', [3]], ['UserService.WatchUsers', [3]]]],
    ])
    expect(inUse([0, 2, 3])).toEqual([
      ['host', [['save_file', [0]]]],
      ['api', [['UserService.GetUser', [3]], ['UserService.WatchUsers', [3]]]],
    ])
    expect(inUse([1])).toEqual([
      ['host', [['greet', [1]], ['save_file', [1]]]],
      ['api', []],
    ])
  })

  it('draws one port arrow per calling container, listing only the contracts called from it', () => {
    const s = systemsOf(INDEX, buildModel(SNAPSHOT, []))
    expect(s.portLinks.map((l) => [l.container, l.system, l.contracts.map((c) => c.id)])).toEqual([
      [3, 'host', ['greet', 'save_file']],
      [1, 'api', ['UserService.GetUser', 'UserService.WatchUsers']],
      [2, 'host', ['save_file']],
    ])
    expect(s.portLink(2, 'host')?.contracts[0].callers).toEqual([{ module: 0, line: 9, owner: null }])
    expect(s.portLink(2, 'api')).toBeNull()
  })
})
