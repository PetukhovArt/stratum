import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiStatusError, fetchEdge, fetchSource } from './api'

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
  afterEach(() => vi.unstubAllGlobals())

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

describe('fetchSource', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('encodes the path and rejects with the status of a missing file', async () => {
    const fetch = answer(404, {})
    vi.stubGlobal('fetch', fetch)
    const pending = fetchSource({ path: 'src/a b.ts', line: 7, context: 3 })
    await expect(pending).rejects.toMatchObject({ status: 404 })
    expect(fetch).toHaveBeenCalledWith('/api/source?path=src%2Fa%20b.ts&line=7&context=3')
  })
})
