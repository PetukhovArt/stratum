import { describe, expect, it } from 'vitest'
import { EdgeImport } from '../lib/api'
import { contractOf } from './contract'

const USER = { path: 'src/entities/user/model.ts', line: 4 }
const ADMIN_USER = { path: 'src/entities/admin/model.ts', line: 9 }

const imp = (file: string, line: number, names: EdgeImport['names'], isTypeOnly = false): EdgeImport => ({
  file,
  line,
  isTypeOnly,
  kind: 'static',
  names,
})

describe('contractOf', () => {
  it('unites a name imported by two files into one entry counted per importer file', () => {
    const contract = contractOf([
      imp('src/pages/b.ts', 2, [{ name: 'User', isTypeOnly: false, decl: USER }]),
      imp('src/pages/a.ts', 7, [{ name: 'User', isTypeOnly: true, decl: USER }]),
      imp('src/pages/a.ts', 1, [{ name: 'User', isTypeOnly: true, decl: USER }]),
    ])
    expect(contract.names).toEqual([
      { name: 'User', isTypeOnly: false, decl: USER, importers: 2, site: { path: 'src/pages/a.ts', line: 1 } },
    ])
    expect(contract.importers.map((g) => [g.file, g.imports.map((i) => i.line)])).toEqual([
      ['src/pages/a.ts', [1, 7]],
      ['src/pages/b.ts', [2]],
    ])
  })

  it('marks a name type-only when its whole import is `import type`', () => {
    const contract = contractOf([imp('src/a.ts', 1, [{ name: 'User', isTypeOnly: false, decl: USER }], true)])
    expect(contract.names[0].isTypeOnly).toBe(true)
  })

  it('keeps one name declared in two places, and an unresolved name, as separate entries', () => {
    const contract = contractOf([
      imp('src/a.ts', 1, [
        { name: 'User', isTypeOnly: false, decl: USER },
        { name: 'User', isTypeOnly: false, decl: ADMIN_USER },
        { name: 'User', isTypeOnly: false, decl: null },
      ]),
    ])
    expect(contract.names.map((n) => n.decl)).toEqual([null, ADMIN_USER, USER])
  })
})
