import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiStatusError, fetchEdge, fetchFeatures, fetchSource, fetchSystems } from './api'

const answer = (status: number, body: unknown) =>
  vi.fn(async () => new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } }))

const EDGE = {
  imports: [
    {
      file: 'src/pages/cart/ui.ts',
      line: 3,
      type_only: false,
      kind: 'static',
      names: [
        { name: 'User', type_only: true, decl: { path: 'src/entities/user/User.ts', line: 1 } },
        { name: 'lost', type_only: false, decl: null },
      ],
    },
  ],
}

describe('fetchEdge', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('asks the server once per module edge and parses the names', async () => {
    const fetch = answer(200, EDGE)
    vi.stubGlobal('fetch', fetch)
    const first = await fetchEdge({ level: 'module', from: 1, to: 2 })
    const second = await fetchEdge({ level: 'module', from: 1, to: 2 })
    expect(fetch).toHaveBeenCalledTimes(1)
    expect(fetch).toHaveBeenCalledWith('/api/edge?from=1&to=2')
    expect(second).toBe(first)
    expect(first[0].names).toEqual([
      { name: 'User', isTypeOnly: true, decl: { path: 'src/entities/user/User.ts', line: 1 } },
      { name: 'lost', isTypeOnly: false, decl: null },
    ])
  })

  it('addresses a container bundle by container ids', async () => {
    const fetch = answer(200, { imports: [] })
    vi.stubGlobal('fetch', fetch)
    await fetchEdge({ level: 'container', from: 1, to: 2 })
    expect(fetch).toHaveBeenCalledWith('/api/edge?from_container=1&to_container=2')
  })

  it('retries an edge whose request failed', async () => {
    vi.stubGlobal('fetch', answer(500, {}))
    await expect(fetchEdge({ level: 'module', from: 3, to: 4 })).rejects.toBeInstanceOf(ApiStatusError)
    vi.stubGlobal('fetch', answer(200, EDGE))
    await expect(fetchEdge({ level: 'module', from: 3, to: 4 })).resolves.toHaveLength(1)
  })
})

describe('fetchFeatures', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('parses the wire shape of /api/features into features and the CLI selection', async () => {
    vi.stubGlobal(
      'fetch',
      answer(200, {
        features: [{ id: 'cart', name: 'Cart', entry: [1], modules: [1, 2] }],
        shared: [],
        selected: ['cart'],
      }),
    )
    await expect(fetchFeatures()).resolves.toEqual({
      features: [{ id: 'cart', name: 'Cart', entry: [1], modules: [1, 2] }],
      selected: ['cart'],
    })
  })
})

describe('fetchSystems', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('parses the wire shape of /api/systems, splitting off the self system', async () => {
    vi.stubGlobal(
      'fetch',
      answer(200, {
        systems: [
          { id: 'ui', name: 'Web UI', self: true, contracts_total: 0 },
          { id: 'api', name: 'Backend', self: false, contracts_total: 4 },
          { id: 'billing', name: 'billing', self: false, contracts_total: 0, unavailable: 'root is not a directory' },
        ],
        channels: [
          {
            from: 'ui',
            to: 'api',
            contracts: [
              {
                id: 'UserService.GetUser',
                signature: 'rpc GetUser(GetUserRequest) returns (User)',
                decl: { path: 'api:user.proto', line: 7 },
                callers: [{ module: 3, line: 7, owner: 'getUser', features: ['checkout'] }],
              },
              { id: 'UserService.forgetUser', signature: null, decl: null, callers: [{ module: 3, line: 12, owner: null }] },
            ],
            unattributed: 1,
          },
        ],
      }),
    )
    const index = await fetchSystems()
    expect(index.self).toEqual({ id: 'ui', name: 'Web UI', contractsTotal: 0, unavailableReason: null })
    expect(index.others.map((s) => [s.id, s.unavailableReason])).toEqual([
      ['api', null],
      ['billing', 'root is not a directory'],
    ])
    expect(index.channels[0]).toMatchObject({ from: 'ui', to: 'api', unattributedCalls: 1 })
    expect(index.channels[0].contracts[0].callers[0].features).toEqual(['checkout'])
    expect(index.channels[0].contracts[1]).toEqual({
      id: 'UserService.forgetUser',
      signature: null,
      decl: null,
      callers: [{ module: 3, line: 12, owner: null, features: null }],
    })
  })
})

describe('fetchSource', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('encodes the path and rejects with the status of a missing file', async () => {
    const fetch = answer(404, {})
    vi.stubGlobal('fetch', fetch)
    const pending = fetchSource({ path: 'src/a b.ts', line: 7, context: 3 })
    await expect(pending).rejects.toMatchObject({ status: 404 })
    expect(fetch).toHaveBeenCalledWith('/api/source?path=src%2Fa%20b.ts&line=7&context=3')
  })
})
