import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const imp = (file: string, line: number, name: string) => ({
  file,
  line,
  type_only: false,
  kind: 'static',
  names: [{ name, type_only: false, decl: { path: 'src/shared/api/index.ts', line: 1 } }],
})

const SITE: Record<string, unknown> = {
  './data/edges.json': {
    '1': { '3': { imports: [imp('src/b.ts', 4, 'b3')] }, '4': { imports: [imp('src/b.ts', 2, 'b4')] } },
    '2': { '3': { imports: [imp('src/a.ts', 9, 'a3')] }, '5': { imports: [imp('src/a.ts', 1, 'out')] } },
  },
  './data/members.json': { '10': [1, 2], '20': [3, 4], '30': [5] },
  './data/source/index.json': { 'src/a.ts': 0, 'host:src/commands.rs': 1 },
  './data/source/0.json': { path: 'src/a.ts', lines: ['l1', 'l2', 'l3', 'l4', 'l5', 'l6'] },
}

const serveSite = () =>
  vi.fn(async (url: string) =>
    url in SITE ? new Response(JSON.stringify(SITE[url]), { status: 200 }) : new Response('not found', { status: 404 }),
  )

const loadApi = async (flags: { source: boolean; systems: boolean; drift: boolean }) => {
  window.__STRATUM_STATIC__ = flags
  vi.resetModules()
  return import('./api')
}

describe('a static site from stratum-lint export', () => {
  let fetch: ReturnType<typeof serveSite>
  beforeEach(() => {
    fetch = serveSite()
    vi.stubGlobal('fetch', fetch)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    delete window.__STRATUM_STATIC__
  })

  it('joins a container bundle from the module edges between the two member sets, by file then line', async () => {
    const api = await loadApi({ source: false, systems: false, drift: false })
    const bundle = await api.fetchEdge({ level: 'container', from: 10, to: 20 })
    expect(bundle.map((i) => `${i.file}:${i.line}`)).toEqual(['src/a.ts:9', 'src/b.ts:2', 'src/b.ts:4'])
    await expect(api.fetchEdge({ level: 'module', from: 1, to: 4 })).resolves.toHaveLength(1)
    await expect(api.fetchEdge({ level: 'module', from: 4, to: 1 })).resolves.toEqual([])
  })

  it('cuts the same source window as the server and 404s a path that was not exported', async () => {
    const api = await loadApi({ source: true, systems: false, drift: false })
    await expect(api.fetchSource({ path: 'src/a.ts', line: 2, context: 2 })).resolves.toEqual({
      path: 'src/a.ts',
      start: 1,
      lines: ['l1', 'l2', 'l3', 'l4'],
      total: 6,
    })
    await expect(api.fetchSource({ path: 'src/a.ts', line: 5, context: 1 })).resolves.toMatchObject({
      start: 4,
      lines: ['l4', 'l5', 'l6'],
    })
    await expect(api.fetchSource({ path: 'src/gone.ts', line: 1, context: 1 })).rejects.toMatchObject({ status: 404 })
  })

  it('rejects source without --with-source and an optional answer that was not exported, without fetching', async () => {
    const api = await loadApi({ source: false, systems: false, drift: false })
    await expect(api.fetchSource({ path: 'src/a.ts', line: 1, context: 1 })).rejects.toBeInstanceOf(
      api.SourceNotExportedError,
    )
    await expect(api.fetchSystems()).rejects.toMatchObject({ status: 404 })
    await expect(api.fetchDrift()).rejects.toMatchObject({ status: 404 })
    expect(fetch).not.toHaveBeenCalled()
  })
})
