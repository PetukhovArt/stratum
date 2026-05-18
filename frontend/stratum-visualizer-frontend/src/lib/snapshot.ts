import { GraphSnapshot, SUPPORTED_SNAPSHOT_VERSION } from '../types'

export class SnapshotVersionMismatchError extends Error {
  constructor(actual: number) {
    super(
      `Unsupported snapshot version ${actual}; this visualizer build expects ${SUPPORTED_SNAPSHOT_VERSION}. ` +
        `Update the visualizer to match \`stratum-lint --version\`.`,
    )
    this.name = 'SnapshotVersionMismatchError'
  }
}

export const loadSnapshot = async (url: string): Promise<GraphSnapshot> => {
  const resp = await fetch(url)
  if (!resp.ok) {
    throw new Error(`Snapshot fetch failed: ${resp.status}`)
  }
  const data = (await resp.json()) as GraphSnapshot
  if (data.version !== SUPPORTED_SNAPSHOT_VERSION) {
    throw new SnapshotVersionMismatchError(data.version)
  }
  return data
}
