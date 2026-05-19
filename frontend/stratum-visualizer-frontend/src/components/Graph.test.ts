import { describe, expect, it } from 'vitest'
import { globToRegex, normalizePath } from './Graph'

const match = (glob: string, path: string): boolean => {
  const fn = globToRegex(glob)
  if (!fn) throw new Error(`globToRegex returned null for "${glob}"`)
  return fn(path)
}

describe('normalizePath', () => {
  it('replaces backslashes with forward slashes', () => {
    expect(normalizePath('src\\features\\cart\\index.ts')).toBe('src/features/cart/index.ts')
  })
  it('leaves forward-slash paths unchanged', () => {
    expect(normalizePath('src/features/cart/index.ts')).toBe('src/features/cart/index.ts')
  })
})

describe('globToRegex', () => {
  it('returns null for empty input', () => {
    expect(globToRegex('')).toBeNull()
    expect(globToRegex('   ')).toBeNull()
    expect(globToRegex(',  ,')).toBeNull()
  })

  it('matches a literal path', () => {
    expect(match('src/features/cart/index.ts', 'src/features/cart/index.ts')).toBe(true)
    expect(match('src/features/cart/index.ts', 'src/features/cart/other.ts')).toBe(false)
  })

  it('single * does not cross /', () => {
    expect(match('src/*.ts', 'src/foo.ts')).toBe(true)
    expect(match('src/*.ts', 'src/foo/bar.ts')).toBe(false)
  })

  it('bare token without metachars is a case-insensitive substring match', () => {
    expect(match('gis', 'src/renderer/features/gis/index.ts')).toBe(true)
    expect(match('gis', 'src/foo/gis-utils.ts')).toBe(true)
    expect(match('GIS', 'src/foo/gis-utils.ts')).toBe(true)
    expect(match('gis', 'src/foo/bar.ts')).toBe(false)
  })

  it('auto-prefixes patterns so they match any tail of the path', () => {
    const winAbs = 'D:/web-projects/proj/src/renderer/features/gis/index.ts'
    expect(match('src/renderer/features/gis/**', winAbs)).toBe(true)
    expect(match('src/renderer/features/gis/**', 'src/renderer/features/gis/index.ts')).toBe(true)
    expect(match('src/renderer/features/gis/**', 'src/renderer/features/other/index.ts')).toBe(false)
  })

  it('auto-prefix works with backslash-separated paths from Windows snapshots', () => {
    expect(match('src/renderer/features/gis/**', 'D:\\web-projects\\proj\\src\\renderer\\features\\gis\\index.ts')).toBe(true)
  })

  it('trailing ** matches everything under prefix', () => {
    expect(match('src/**', 'src/foo.ts')).toBe(true)
    expect(match('src/**', 'src/a/b/c.ts')).toBe(true)
    expect(match('src/**', 'other/foo.ts')).toBe(false)
  })

  it('middle ** matches zero or more segments', () => {
    expect(match('src/**/test.ts', 'src/test.ts')).toBe(true)
    expect(match('src/**/test.ts', 'src/a/test.ts')).toBe(true)
    expect(match('src/**/test.ts', 'src/a/b/test.ts')).toBe(true)
    expect(match('src/**/test.ts', 'src/test.tsx')).toBe(false)
  })

  it('normalizes Windows-style paths transparently', () => {
    expect(match('src/features/**', 'src\\features\\cart\\index.ts')).toBe(true)
    expect(match('src\\features\\**', 'src/features/cart/index.ts')).toBe(true)
  })

  it('comma-separated patterns OR together', () => {
    expect(match('src/entities/**, src/features/**', 'src/entities/user/model.ts')).toBe(true)
    expect(match('src/entities/**, src/features/**', 'src/features/cart/index.ts')).toBe(true)
    expect(match('src/entities/**, src/features/**', 'src/shared/utils.ts')).toBe(false)
  })

  it('!-prefixed patterns exclude from positive set', () => {
    expect(match('src/**, !src/shared/**', 'src/features/cart.ts')).toBe(true)
    expect(match('src/**, !src/shared/**', 'src/shared/utils.ts')).toBe(false)
  })

  it('only negative patterns include everything except matches', () => {
    expect(match('!src/shared/**', 'src/features/cart.ts')).toBe(true)
    expect(match('!src/shared/**', 'src/shared/utils.ts')).toBe(false)
    expect(match('!src/shared/**', 'other/file.ts')).toBe(true)
  })

  it('? matches a single character', () => {
    expect(match('src/file?.ts', 'src/file1.ts')).toBe(true)
    expect(match('src/file?.ts', 'src/file12.ts')).toBe(false)
  })

  it('escapes regex metachars in literal segments', () => {
    expect(match('src/(foo).ts', 'src/(foo).ts')).toBe(true)
    expect(match('src/(foo).ts', 'src/xfoox.ts')).toBe(false)
  })
})
