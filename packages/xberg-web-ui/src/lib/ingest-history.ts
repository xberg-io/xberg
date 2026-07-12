import type { IngestHistoryEntry } from "./types.js";

const DB_NAME = "xberg-web-ui";
const DB_VERSION = 1;
const STORE_NAME = "ingest-history";

function keyFor(collection: string, externalId: string): string {
  return `${collection}::${externalId}`;
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: "key" });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("failed to open indexedDB"));
  });
}

export async function putHistoryEntry(entry: IngestHistoryEntry): Promise<void> {
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, "readwrite");
      tx.objectStore(STORE_NAME).put({ key: keyFor(entry.collection, entry.externalId), ...entry });
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error("failed to write ingest history entry"));
      tx.onabort = () => reject(new Error("transaction aborted"));
    });
  } finally {
    db.close();
  }
}

export async function getHistoryEntry(collection: string, externalId: string): Promise<IngestHistoryEntry | null> {
  const db = await openDb();
  try {
    return await new Promise<IngestHistoryEntry | null>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, "readonly");
      const req = tx.objectStore(STORE_NAME).get(keyFor(collection, externalId));
      req.onsuccess = () => {
        if (!req.result) {
          resolve(null);
          return;
        }
        const { key, ...entry } = req.result as IngestHistoryEntry & { key: string };
        resolve(entry);
      };
      req.onerror = () => reject(req.error ?? new Error("failed to read ingest history entry"));
      tx.onabort = () => reject(new Error("transaction aborted"));
    });
  } finally {
    db.close();
  }
}

export async function listHistory(collection?: string): Promise<IngestHistoryEntry[]> {
  const db = await openDb();
  try {
    const all = await new Promise<IngestHistoryEntry[]>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, "readonly");
      const req = tx.objectStore(STORE_NAME).getAll();
      req.onsuccess = () => {
        const rows = (req.result as Array<IngestHistoryEntry & { key: string }>).map(({ key, ...entry }) => entry);
        resolve(rows);
      };
      req.onerror = () => reject(req.error ?? new Error("failed to list ingest history"));
      tx.onabort = () => reject(new Error("transaction aborted"));
    });
    return collection ? all.filter((e) => e.collection === collection) : all;
  } finally {
    db.close();
  }
}

export async function listFolders(): Promise<string[]> {
  const all = await listHistory();
  return Array.from(new Set(all.map((e) => e.collection))).sort();
}
