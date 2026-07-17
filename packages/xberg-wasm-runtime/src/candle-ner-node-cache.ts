/**
 * Node-only disk cache for `candle-ner.ts`'s model downloads. Split into its
 * own file, dynamically imported only from the Node branch of
 * `fetchWithCache`, so the browser webpack bundle never has to resolve
 * `node:fs`/`node:path` -- a static top-level import of these in
 * `candle-ner.ts` broke the browser build (`UnhandledSchemeError: Reading
 * from "node:fs" is not handled by plugins`) because `factory.ts` statically
 * imports `candle-ner.ts`. Mirrors the existing `store.ts` -> dynamic
 * `import("./store-node.js")` pattern used for the same reason.
 */
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

/** `join(nodeCachePath, "candle-ner")`, OS-correct path separator. */
export function candleNerCacheDir(nodeCachePath: string): string {
	return join(nodeCachePath, "candle-ner");
}

/** Read `<nodeCacheDir>/<fileName>` if present and non-empty, else `null`. */
export function readCached(nodeCacheDir: string, fileName: string): Uint8Array | null {
	const localPath = join(nodeCacheDir, fileName);
	// A zero-byte file means a previous download was interrupted -- treat it
	// as absent so it retries, rather than "successfully" loading an empty
	// model file.
	if (existsSync(localPath) && statSync(localPath).size > 0) {
		return new Uint8Array(readFileSync(localPath));
	}
	return null;
}

/** Write `bytes` to `<nodeCacheDir>/<fileName>`, creating parent dirs. */
export function writeCached(nodeCacheDir: string, fileName: string, bytes: Uint8Array): void {
	const localPath = join(nodeCacheDir, fileName);
	mkdirSync(dirname(localPath), { recursive: true });
	writeFileSync(localPath, bytes);
}
