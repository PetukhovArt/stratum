import { GraphSnapshot, SUPPORTED_SNAPSHOT_VERSION } from '../types'

export class SnapshotVersionMismatchError extends Error {
  constructor(actual: unknown) {
    super(
      `Unsupported snapshot version ${String(actual)}; this visualizer build expects ${SUPPORTED_SNAPSHOT_VERSION}. ` +
        `Update the visualizer to match \`stratum-lint --version\`.`,
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

type FieldType = 'number' | 'string' | 'array' | 'number|null'

const MODULE_FIELDS: Record<string, FieldType> = {
  id: 'number',
  path: 'string',
  container: 'number',
  layer: 'number',
  stage: 'number',
}
const CONTAINER_FIELDS: Record<string, FieldType> = {
  id: 'number',
  name: 'string',
  layer: 'number',
  parent: 'number|null',
}
const LAYER_FIELDS: Record<string, FieldType> = { id: 'number', name: 'string', depends_on: 'array' }
const EDGE_FIELDS: Record<string, FieldType> = { from: 'number', to: 'number', kind: 'string' }

const hasType = (value: unknown, type: FieldType): boolean => {
  if (type === 'array') return Array.isArray(value)
  if (type === 'number|null') return value === null || typeof value === 'number'
  return typeof value === type
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
