import { Violation } from '../types'

export const loadViolations = async (url: string): Promise<Violation[]> => {
  const resp = await fetch(url)
  if (!resp.ok) throw new Error(`Violations fetch failed: ${resp.status}`)
  return (await resp.json()) as Violation[]
}
