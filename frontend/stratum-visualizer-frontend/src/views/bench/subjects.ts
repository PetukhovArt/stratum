import { Model } from '../../render/model'

export type SubjectRole = 'fan-in' | 'fan-out' | 'findings' | 'private'

export interface Subject {
  role: SubjectRole
  id: number
  path: string
}

export interface Subjects {
  hub: Subject
  list: readonly Subject[]
}

const edgeFindingCount = (m: Model, id: number): number =>
  m.out(id).reduce((n, to) => n + m.findingsOnEdge(id, to).length, 0) +
  m.inn(id).reduce((n, from) => n + m.findingsOnEdge(from, id).length, 0)

const argmax = (ids: readonly number[], score: (id: number) => number): number | null => {
  let best: number | null = null
  let bestScore = 0
  for (const id of ids) {
    const s = score(id)
    if (s > bestScore) {
      best = id
      bestScore = s
    }
  }
  return best
}

export const pickSubjects = (m: Model): Subjects => {
  const ids = [...m.modules.keys()].sort((a, b) => a - b)
  const degree = (id: number) => m.out(id).length + m.inn(id).length
  const zoned = ids.filter((id) => typeof m.modules.get(id)?.zone === 'number')
  const picks: [SubjectRole, number | null][] = [
    ['fan-in', argmax(ids, (id) => m.inn(id).length) ?? ids[0] ?? null],
    ['fan-out', argmax(ids, (id) => m.out(id).length)],
    ['findings', argmax(ids, (id) => edgeFindingCount(m, id))],
    ['private', zoned.length ? argmax(zoned, degree) : null],
  ]
  const seen = new Set<number>()
  const list: Subject[] = []
  for (const [role, id] of picks) {
    if (id === null || seen.has(id)) continue
    seen.add(id)
    list.push({ role, id, path: m.modules.get(id)!.path })
  }
  if (!list.length) throw new Error('The bench needs a snapshot with at least one module')
  return { hub: list[0], list }
}
