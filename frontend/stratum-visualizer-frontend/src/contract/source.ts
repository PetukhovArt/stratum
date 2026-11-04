import { SourceText } from '../lib/api'

export interface SourceRow {
  line: number
  text: string
  html: string | null
  isMarked: boolean
}

const SPAN_OR_BREAK = /<\/span>|<span[^>]*>|\n/g

export const balancedLinesOf = (html: string): string[] => {
  const lines: string[] = []
  const open: string[] = []
  let line = ''
  let last = 0
  for (const match of html.matchAll(SPAN_OR_BREAK)) {
    const tag = match[0]
    line += html.slice(last, match.index)
    last = (match.index ?? 0) + tag.length
    if (tag === '\n') {
      lines.push(line + '</span>'.repeat(open.length))
      line = open.join('')
    } else if (tag === '</span>') {
      open.pop()
      line += tag
    } else {
      open.push(tag)
      line += tag
    }
  }
  lines.push(line + html.slice(last))
  return lines
}

export const rowsOf = (source: SourceText, mark: number, html: readonly string[] | null): SourceRow[] =>
  source.lines.map((text, i) => {
    const line = source.start + i
    return { line, text, html: html?.[i] ?? null, isMarked: line === mark }
  })
