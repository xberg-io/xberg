import { type RagStore, openSqlite } from "xberg-rag-node";
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
  _store = await openSqlite("default", dbPath);
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