const writes = new Map<number, Promise<void>>();

/** Preserve Enter/Tab order while allowing the user to continue editing immediately. */
export function queueFieldWrite<T>(projectId: number, operation: () => Promise<T>) {
  const result = (writes.get(projectId) ?? Promise.resolve()).then(operation);
  // A rejected value must not prevent the next valid edit from being written.
  const settled = result.then(() => {}, () => {});
  writes.set(projectId, settled);
  void settled.then(() => { if (writes.get(projectId) === settled) writes.delete(projectId); });
  return result;
}

/** Save/close must include edits still waiting for backend validation. */
export async function awaitFieldWrites(projectId: number) {
  while (writes.has(projectId)) await writes.get(projectId);
}
