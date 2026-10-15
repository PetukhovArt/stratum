import { CONTAINER_KINDS, GraphSnapshot, SUPPORTED_SNAPSHOT_VERSION } from '../types'

const upgradeHint = (actual: unknown): string =>
  typeof actual === 'number' && actual < SUPPORTED_SNAPSHOT_VERSION
    ? `The snapshot comes from an older stratum-lint; upgrade stratum-lint and regenerate it.`
    : `Update the visualizer to match \`stratum-lint --version\`.`

export class SnapshotVersionMismatchError extends Error {
  constructor(actual: unknown) {
    super(
      `Unsupported snapshot version ${String(actual)}; this visualizer build expects ${SUPPORTED_SNAPSHOT_VERSION}. ` +
        upgradeHint(actual),
    )
    this.name = 'SnapshotVersionMismatchError'
  }
}

export class SnapshotShapeError extends Error {
  constructor(where: string) {
    super(`Malformed snapshot: ${where}`)
    this.name = 'SnapshotShapeError'
  }
}

type FieldType = 'number' | 'string' | 'array' | 'number|null' | 'rank 1-4' | 'container kind'

const MODULE_FIELDS: Record<string, FieldType> = {
  id: 'number',
  path: 'string',
  container: 'number',
  layer: 'number',
  purity: 'rank 1-4',
}
const CONTAINER_FIELDS: Record<string, FieldType> = {
  id: 'number',
  name: 'string',
  path: 'string',
  kind: 'container kind',
  layer: 'number',
  parent: 'number|null',
  stage: 'rank 1-4',
  entry: 'number|null',
}
const LAYER_FIELDS: Record<string, FieldType> = { id: 'number', name: 'string', depends_on: 'array' }
const EDGE_FIELDS: Record<string, FieldType> = { from: 'number', to: 'number', kind: 'string' }

const CONTAINER_KIND_SET: ReadonlySet<unknown> = new Set(CONTAINER_KINDS)

const hasType = (value: unknown, type: FieldType): boolean => {
  switch (type) {
    case 'array':
      return Array.isArray(value)
    case 'number|null':
      return value === null || typeof value === 'number'
    case 'rank 1-4':
      return Number.isInteger(value) && (value as number) >= 1 && (value as number) <= 4
    case 'container kind':
      return CONTAINER_KIND_SET.has(value)
    default:
      return typeof value === type
  }
}

const assertRecords = (raw: Record<string, unknown>, key: string, fields: Record<string, FieldType>): void => {
  const list = raw[key]
  if (!Array.isArray(list)) {
    throw new SnapshotShapeError(`\`${key}\` is not an array`)
  }
  list.forEach((item: unknown, idx) => {
    if (typeof item !== 'object' || item === null) {
      throw new SnapshotShapeError(`${key}[${idx}] is not an object`)
    }
    const record = item as Record<string, unknown>
    for (const [field, type] of Object.entries(fields)) {
      if (!hasType(record[field], type)) {
        throw new SnapshotShapeError(`${key}[${idx}].${field} is not ${type}`)
      }
    }
  })
}

export const parseSnapshot = (raw: unknown): GraphSnapshot => {
  if (typeof raw !== 'object' || raw === null) {
    throw new SnapshotShapeError('not an object')
  }
  const record = raw as Record<string, unknown>
  if (record.version !== SUPPORTED_SNAPSHOT_VERSION) {
    throw new SnapshotVersionMismatchError(record.version)
  }
  assertRecords(record, 'modules', MODULE_FIELDS)
  assertRecords(record, 'containers', CONTAINER_FIELDS)
  assertRecords(record, 'layers', LAYER_FIELDS)
  assertRecords(record, 'edges', EDGE_FIELDS)
  return record as unknown as GraphSnapshot
}

export const loadSnapshot = async (url: string): Promise<GraphSnapshot> => {
  const resp = await fetch(url)
  if (!resp.ok) {
    throw new Error(`Snapshot fetch failed: ${resp.status}`)
  }
  return parseSnapshot(await resp.json())
}
