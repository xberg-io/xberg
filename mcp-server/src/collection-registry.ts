/**
 * Session-local registry of collection names created via `create_collection`.
 *
 * The wasm runtime's `VectorStoreInterface` has no `listCollections()` method
 * (see `packages/xberg-wasm-runtime/src/types.ts`), so `list_collections`
 * cannot enumerate the backing store directly. This in-memory Set mirrors the
 * bookkeeping the native store previously did in `store.ts`, scoped to this
 * process only.
 */
const _collections = new Set<string>();

export function trackCollection(name: string): void {
  _collections.add(name);
}

export function untrackCollection(name: string): void {
  _collections.delete(name);
}

export function listTrackedCollections(): string[] {
  return [..._collections];
}
