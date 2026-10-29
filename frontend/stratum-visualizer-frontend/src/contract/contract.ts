import { EdgeImport, Loc } from '../lib/api'

export interface UsedName {
  name: string
  isTypeOnly: boolean
  decl: Loc | null
  importers: number
  site: Loc
}

export interface ImporterGroup {
  file: string
  imports: EdgeImport[]
}

export interface Contract {
  names: UsedName[]
  importers: ImporterGroup[]
}

const nameKey = (name: string, decl: Loc | null): string => `${name}@${decl ? `${decl.path}:${decl.line}` : '?'}`

const byText = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0)

export const contractOf = (imports: readonly EdgeImport[]): Contract => {
  const byFile = new Map<string, EdgeImport[]>()
  for (const imp of imports) byFile.set(imp.file, [...(byFile.get(imp.file) ?? []), imp])
  const importers = [...byFile.entries()]
    .sort(([a], [b]) => byText(a, b))
    .map(([file, list]) => ({ file, imports: list.slice().sort((a, b) => a.line - b.line) }))

  const used = new Map<string, UsedName & { files: Set<string> }>()
  for (const group of importers) {
    for (const imp of group.imports) {
      for (const n of imp.names) {
        const isTypeOnly = imp.isTypeOnly || n.isTypeOnly
        const key = nameKey(n.name, n.decl)
        const hit = used.get(key)
        if (hit) {
          hit.isTypeOnly &&= isTypeOnly
          hit.files.add(imp.file)
          continue
        }
        used.set(key, {
          name: n.name,
          isTypeOnly,
          decl: n.decl,
          importers: 0,
          site: { path: imp.file, line: imp.line },
          files: new Set([imp.file]),
        })
      }
    }
  }
  const names = [...used.values()]
    .map(({ files, ...name }) => ({ ...name, importers: files.size }))
    .sort((a, b) => byText(a.name, b.name) || byText(a.decl?.path ?? '', b.decl?.path ?? ''))
  return { names, importers }
}
