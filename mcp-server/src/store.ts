import { RagStore } from "xberg-rag-node";
import { homedir } from "os";
import { join } from "path";
import { mkdirSync } from "fs";

function defaultStorePath(): string {
  const base =
    process.platform === "win32"
      ? process.env.APPDATA ?? join(homedir(), "AppData", "Roaming")
      : join(homedir(), ".local", "share");
  return join(base, "xberg", "store.db");
}

let _store: RagStore | null = null;
let _collections: Set<string> = new Set();

export function getCacheDir(): string {
  const base =
    process.platform === "win32"
      ? process.env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local")
      : join(homedir(), ".cache");
  return join(process.env.XBERG_CACHE_DIR ?? base, "xberg");
}

export async function getStore(): Promise<RagStore> {
  if (_store !== null) return _store;
  const dbPath = process.env.XBERG_STORE_PATH ?? defaultStorePath();
  const dir = dbPath.replace(/[/\\][^/\\]+$/, "");
  if (dir) mkdirSync(dir, { recursive: true });
  _store = await RagStore.openSqlite("default", dbPath);
  return _store!;
}

export function trackCollection(name: string): void {
  _collections.add(name);
}

export function untrackCollection(name: string): void {
  _collections.delete(name);
}

export function listTrackedCollections(): string[] {
  return Array.from(_collections);
}

/**
 * Ensure a collection exists with an embedding dimension that matches the
 * vectors the tools will actually produce. The SQLite store rejects any chunk
 * whose embedding length differs from the collection's declared dimension, so
 * creating the collection with the embedder's real output dim (not a hardcoded
 * default) is what keeps the default ingest → query path working.
 *
 * Only creates when absent; a pre-existing collection with a mismatched dim is
 * left to surface a clear EmbeddingDimMismatch error at upsert time.
 */
export async function ensureCollectionWithDim(
  store: RagStore,
  collection: string,
  dim: number,
): Promise<void> {
  if (dim <= 0) {
    throw new Error(`Embedding dimension must be greater than 0, received ${dim}`);
  }

  const existing = await store.getCollection(collection);
  if (existing === null) {
    await store.ensureCollection(
      JSON.stringify({
        name: collection,
        embedding_dim: dim,
        distance_metric: "cosine",
        index_method: "flat",
      }),
    );
  }
}

/**
 * Race a promise against a timeout, rejecting with a clear error if the
 * underlying operation (e.g. native embedding inference) does not settle.
 * Prevents the server from hanging indefinitely when an ONNX model is missing
 * or slow to load.
 */
export function withTimeout<T>(p: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    let timedOut = false;
    const t = setTimeout(() => {
      timedOut = true;
      console.warn(`${label} timed out after ${ms}ms; underlying work is still running`);
      reject(new Error(`${label} timed out after ${ms}ms (model may be missing or slow to load)`));
    }, ms);
    p.then(
      (v) => {
        clearTimeout(t);
        if (timedOut) {
          console.warn(`${label} resolved after timing out`);
          return;
        }
        resolve(v);
      },
      (e) => {
        clearTimeout(t);
        if (timedOut) {
          console.warn(`${label} rejected after timing out`, e);
          return;
        }
        reject(e);
      },
    );
  });
}
