import hljs from 'highlight.js/lib/core'
import protobuf from 'highlight.js/lib/languages/protobuf'
import rust from 'highlight.js/lib/languages/rust'
import typescript from 'highlight.js/lib/languages/typescript'
import { balancedLinesOf } from './source'

hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('protobuf', protobuf)

const LANGUAGE_BY_EXTENSION: Readonly<Record<string, string>> = {
  ts: 'typescript',
  tsx: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  js: 'typescript',
  jsx: 'typescript',
  mjs: 'typescript',
  cjs: 'typescript',
  rs: 'rust',
  proto: 'protobuf',
}

export const highlightedLinesOf = (path: string, lines: readonly string[]): string[] | null => {
  const language = LANGUAGE_BY_EXTENSION[path.slice(path.lastIndexOf('.') + 1)]
  if (!language) return null
  return balancedLinesOf(hljs.highlight(lines.join('\n'), { language, ignoreIllegals: true }).value)
}
