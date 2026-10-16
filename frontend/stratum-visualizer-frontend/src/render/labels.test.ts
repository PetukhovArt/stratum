import { describe, expect, it } from 'vitest'
import { disambiguate, ellipsizePath } from './labels'

describe('ellipsizePath', () => {
  it('drops middle directories before it cuts into the file name', () => {
    expect(ellipsizePath('src/renderer/entities/user/profile/composition.ts', 30)).toBe('…/user/profile/composition.ts')
    expect(ellipsizePath('windows-sync/peer-coordination.ts', 30)).toBe('windows-…/peer-coordination.ts')
  })
})

describe('disambiguate', () => {
  it('extends only colliding names with the shortest parent path that tells them apart', () => {
    const labels = disambiguate([
      { key: 'widgets', label: 'widgets', path: 'src/renderer/widgets' },
      { key: 'widgets', label: 'widgets', path: 'src/legacy/widgets' },
      { key: 'core', label: 'core', path: 'src/core' },
    ])
    expect(labels).toEqual(['renderer/widgets', 'legacy/widgets', 'core'])
  })
})
