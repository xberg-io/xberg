import { homedir } from "os";
import { join } from "path";

/**
 * Resolve the on-disk cache directory for downloaded ML models (embedder, NER,
 * OCR, reranker). Honors `XBERG_CACHE_DIR`; otherwise falls back to the
 * platform cache location. This is pure path logic with no native or store
 * dependency — it used to live in the now-removed native `store.ts`.
 */
export function getCacheDir(): string {
  const base =
    process.platform === "win32"
      ? process.env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local")
      : join(homedir(), ".cache");
  return join(process.env.XBERG_CACHE_DIR ?? base, "xberg");
}
