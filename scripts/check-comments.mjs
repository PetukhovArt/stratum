#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const frontendDir = path.join(repoRoot, 'frontend', 'stratum-visualizer-frontend')

const SCRIPT = /\.(?:[cm]?[jt]sx?)$/
const CHECKED = [/\.rs$/, SCRIPT, /\.css$/]
const NOT_CODE = [
  /(?:^|\/)fixtures\//,
  /(?:^|\/)snapshots\//,
  /(?:^|\/)dist\//,
  /(?:^|\/)node_modules\//,
  /^benches\/elk-spike\//,
  /(?:^|\/)[^/]*\.config\.[cm]?[jt]s$/,
]

const DIRECTIVES = [
  /^prettier-ignore\b/,
  /^@vite-ignore\b/,
  /^@jsxImportSource\s/,
  /^[@#]__PURE__$/,
  /^(?:v8|c8|istanbul)\s+ignore\b/,
  /^@vitest-environment\s/,
  /^@stratum-purity\s/,
  /^stratum-ignore\b/,
  /^upstream:\s*\S*https?:\/\/\S+/,
]
const TRIPLE_SLASH_REFERENCE = /^\/\/\/\s*<reference\s/

const git = (...args) => execFileSync('git', ['-c', 'core.quotePath=false', ...args], { cwd: repoRoot, encoding: 'utf8', maxBuffer: 256 << 20 })

const args = process.argv.slice(2)
const optionValue = (name) => {
  const i = args.indexOf(name)
  return i === -1 ? undefined : args[i + 1]
}

const isChecked = (file) => CHECKED.some((rule) => rule.test(file)) && !NOT_CODE.some((rule) => rule.test(file))

const bodyOf = (raw) =>
  raw
    .replace(/^\/\/[/!]?|^\/\*+!?|\*\/$/g, '')
    .split('\n')
    .map((line) => line.replace(/^\s*\*?\s?/, ''))
    .join('\n')
    .trim()

const isAllowed = (raw) => TRIPLE_SLASH_REFERENCE.test(raw) || DIRECTIVES.some((rule) => rule.test(bodyOf(raw)))

const addedLines = () => {
  if (args.includes('--all')) {
    return new Map(
      git('ls-files')
        .split('\n')
        .filter(Boolean)
        .filter((file) => fs.existsSync(path.join(repoRoot, file)))
        .map((file) => [file, null]),
    )
  }
  const target = args.includes('--staged') ? ['--cached'] : [git('merge-base', 'HEAD', optionValue('--base') ?? 'origin/main').trim()]
  const byFile = new Map()
  let lines = null
  let next = 0
  for (const raw of git('diff', '-U0', '--no-color', '--no-ext-diff', '--diff-filter=d', ...target).split('\n')) {
    if (raw.startsWith('+++ ')) {
      const file = raw.slice(4).trim()
      lines = file === '/dev/null' ? null : new Set()
      if (lines) byFile.set(file.replace(/^b\//, ''), lines)
      continue
    }
    const hunk = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(raw)
    if (hunk) {
      next = Number(hunk[1])
      continue
    }
    if (lines && raw.startsWith('+')) lines.add(next++)
  }
  return byFile
}

const read = (file) => {
  if (!args.includes('--staged')) return fs.readFileSync(path.join(repoRoot, file), 'utf8')
  return git('show', `:${file}`)
}

let typescript
const loadTypescript = () => {
  if (typescript) return typescript
  try {
    typescript = createRequire(path.join(frontendDir, 'package.json'))('typescript')
  } catch {
    console.error(`check-comments: run \`npm install\` in ${path.relative(repoRoot, frontendDir)} to check scripts`)
    process.exit(2)
  }
  return typescript
}

const scriptComments = (file, text) => {
  const ts = loadTypescript()
  const kind = /\.[cm]?tsx$|\.jsx$/.test(file) ? ts.ScriptKind.TSX : /\.[cm]?ts$/.test(file) ? ts.ScriptKind.TS : ts.ScriptKind.JS
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, false, kind)
  const ranges = new Map()
  const jsxText = []
  const collect = (found) => {
    for (const range of found ?? []) ranges.set(range.pos, range)
  }
  const visit = (node) => {
    if (node.kind === ts.SyntaxKind.JsxText) jsxText.push(node)
    else {
      collect(ts.getLeadingCommentRanges(text, node.pos))
      collect(ts.getTrailingCommentRanges(text, node.end))
    }
    for (const child of node.getChildren(source)) visit(child)
  }
  visit(source)
  const insideJsxText = (range) => jsxText.some((node) => range.pos >= node.pos && range.pos < node.end)
  return [...ranges.values()].filter((range) => !insideJsxText(range)).map((range) => ({ start: range.pos, end: range.end }))
}

const blockComment = (text, start) => {
  let depth = 1
  let i = start + 2
  while (i < text.length && depth > 0) {
    const pair = text.slice(i, i + 2)
    if (pair === '/*') depth++
    else if (pair === '*/') depth--
    i += pair === '/*' || pair === '*/' ? 2 : 1
  }
  return i
}

const lineComment = (text, start) => {
  const end = text.indexOf('\n', start)
  return end === -1 ? text.length : end
}

const skipQuoted = (text, start, quote) => {
  let i = start + 1
  while (i < text.length && text[i] !== quote) i += text[i] === '\\' ? 2 : 1
  return i + 1
}

const rustComments = (text) => {
  const found = []
  let i = 0
  while (i < text.length) {
    const c = text[i]
    const next = text[i + 1]
    if (c === '/' && next === '/') {
      const end = lineComment(text, i)
      found.push({ start: i, end })
      i = end
    } else if (c === '/' && next === '*') {
      const end = blockComment(text, i)
      found.push({ start: i, end })
      i = end
    } else if ((c === 'r' || c === 'b') && /^b?r#*"/.test(text.slice(i, i + 258)) && !/\w/.test(text[i - 1] ?? '')) {
      const [prefix, hashes] = /^(b?r(#*))"/.exec(text.slice(i)).slice(1)
      const close = text.indexOf(`"${hashes}`, i + prefix.length + 1)
      i = close === -1 ? text.length : close + hashes.length + 1
    } else if (c === '"') {
      i = skipQuoted(text, i, '"')
    } else if (c === "'") {
      const char = /^'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'/u.exec(text.slice(i))
      i += char ? char[0].length : 1
    } else i++
  }
  return found
}

const cssComments = (text) => {
  const found = []
  let i = 0
  while (i < text.length) {
    const c = text[i]
    if (c === '/' && text[i + 1] === '*') {
      const close = text.indexOf('*/', i + 2)
      const end = close === -1 ? text.length : close + 2
      found.push({ start: i, end })
      i = end
    } else if (c === '"' || c === "'") {
      i = skipQuoted(text, i, c)
    } else i++
  }
  return found
}

const lineAt = (starts, offset) => {
  let low = 0
  let high = starts.length - 1
  while (low < high) {
    const mid = (low + high + 1) >> 1
    if (starts[mid] <= offset) low = mid
    else high = mid - 1
  }
  return low + 1
}

const DOCS_AS_DATA = /derive\([^)]*\b(?:JsonSchema|Parser|Subcommand|Args|ValueEnum)\b/

const rustRanges = (text) => {
  const ranges = rustComments(text)
  if (!DOCS_AS_DATA.test(text)) return ranges
  return ranges.filter(({ start }) => !(text.startsWith('///', start) && text[start + 3] !== '/'))
}

const commentsIn = (file, text) => {
  const ranges = file.endsWith('.rs') ? rustRanges(text) : file.endsWith('.css') ? cssComments(text) : scriptComments(file, text)
  const starts = [0]
  for (let i = 0; i < text.length; i++) if (text[i] === '\n') starts.push(i + 1)
  return ranges.map(({ start, end }) => ({
    from: lineAt(starts, start),
    to: lineAt(starts, Math.max(start, end - 1)),
    raw: text.slice(start, end),
  }))
}

const found = []
for (const [file, lines] of addedLines()) {
  if (!isChecked(file) || lines?.size === 0) continue
  for (const comment of commentsIn(file, read(file))) {
    if (isAllowed(comment.raw)) continue
    const line = lines ? [...lines].find((n) => n >= comment.from && n <= comment.to) : comment.from
    if (line !== undefined) found.push(`${file}:${line}: ${comment.raw.split('\n')[0].trim().slice(0, 100)}`)
  }
}

if (found.length === 0) {
  console.log('check-comments: no new comments')
  process.exit(0)
}
console.error(found.join('\n'))
console.error(
  `\ncheck-comments: ${found.length} new comment(s). Code has no comments, doc comments included; only tool directives, \`upstream: <url>\` and the \`///\` text clap and schemars turn into --help and the config schema stay. Say it with a name, a type, a test or a lint rule instead.`,
)
process.exit(1)
