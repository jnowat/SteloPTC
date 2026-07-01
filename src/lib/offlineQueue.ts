// WP-62: offline mutation queue for the PWA build target. Mutations
// attempted while offline are queued in IndexedDB and replayed once
// connectivity returns, in the exact order they were enqueued — which is
// also the order the audit chain's `chain_seq` must see them in, since
// every mutation command writes its own next `chain_seq` on the backend.
//
// The ordering/replay logic below is deliberately factored out as plain,
// dependency-free functions operating on an in-memory array (`enqueue`,
// `replayInOrder`) so it's unit-testable without a real IndexedDB — jsdom
// (this project's Vitest environment) does not implement IndexedDB. The
// `enqueueMutation`/`getQueuedMutations`/etc. wrappers below are the actual
// IndexedDB-backed persistence used by the running app.
//
// Honest scope note: replaying a mutation still calls the same Tauri
// `invoke()` used everywhere else in this app. In a pure browser PWA
// context (no Tauri runtime), there is currently no remote HTTP endpoint
// for `invoke()` to reach — SteloPTC's entire command layer is Tauri IPC
// only today. This queue is therefore a tested, ready-to-wire mechanism;
// making offline mutations actually reach a live backend from a
// browser-only PWA install requires a remote API layer that does not exist
// yet (the same "foundation now, transport later" pattern already used for
// WP-50's PostgreSQL connector and WP-59's S3/SFTP targets).

export interface QueuedMutation {
  id: number;
  command: string;
  args: Record<string, unknown>;
  enqueuedAt: number;
}

/** Pure: appends a new mutation with a caller-assigned monotonic id. */
export function enqueue(
  queue: QueuedMutation[],
  command: string,
  args: Record<string, unknown>,
  nextId: number,
  now: number,
): QueuedMutation[] {
  return [...queue, { id: nextId, command, args, enqueuedAt: now }];
}

export interface ReplayResult {
  succeededIds: number[];
  remaining: QueuedMutation[];
  firstError: { id: number; message: string } | null;
}

/**
 * Pure: replays `queue` strictly in array order (FIFO = enqueue order =
 * chain_seq order), stopping at the first failure. Mutations after a
 * failure are never attempted — applying them out of order relative to one
 * that didn't succeed would violate the chain_seq ordering guarantee the
 * backend's audit chain depends on.
 */
export async function replayInOrder(
  queue: QueuedMutation[],
  invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>,
): Promise<ReplayResult> {
  const succeededIds: number[] = [];
  for (let i = 0; i < queue.length; i++) {
    const mutation = queue[i];
    try {
      await invoke(mutation.command, mutation.args);
      succeededIds.push(mutation.id);
    } catch (e) {
      return {
        succeededIds,
        remaining: queue.slice(i),
        firstError: { id: mutation.id, message: e instanceof Error ? e.message : String(e) },
      };
    }
  }
  return { succeededIds, remaining: [], firstError: null };
}

// ── IndexedDB-backed persistence (used by the running app) ──────────────────

const DB_NAME = 'stelo_offline_queue';
const STORE_NAME = 'mutations';
const DB_VERSION = 1;

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id', autoIncrement: true });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export async function enqueueMutation(command: string, args: Record<string, unknown>): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).add({ command, args, enqueuedAt: Date.now() });
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

export async function getQueuedMutations(): Promise<QueuedMutation[]> {
  const db = await openDb();
  const result = await new Promise<QueuedMutation[]>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readonly');
    const request = tx.objectStore(STORE_NAME).getAll();
    request.onsuccess = () => resolve(request.result as QueuedMutation[]);
    request.onerror = () => reject(request.error);
  });
  db.close();
  return result;
}

export async function removeMutations(ids: number[]): Promise<void> {
  if (ids.length === 0) return;
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    for (const id of ids) store.delete(id);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

/** Loads the persisted queue and replays it, removing whatever succeeded. */
export async function replayQueuedMutations(
  invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>,
): Promise<ReplayResult> {
  const queue = await getQueuedMutations();
  const result = await replayInOrder(queue, invoke);
  await removeMutations(result.succeededIds);
  return result;
}
