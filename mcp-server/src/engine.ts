import type { XbergEngine } from "@xberg-io/xberg-wasm";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import type { InjectionDescriptor } from "xberg-wasm-runtime";
import { homedir } from "os";
import { join } from "path";

let _engine: XbergEngine | null = null;
let _injection: InjectionDescriptor | null = null;

/**
 * Build the `XbergEngine` (B) once, wiring it to C's shared runtime factory.
 *
 * C's single public entry point `createXbergRuntimeFactory` constructs and
 * validates the whole injection descriptor (`{ embedder, store, ner?, ocr? }`)
 * that B's constructor consumes — we do not touch the per-capability factories
 * directly.
 *
 * `CacheConfig` exposes `nodeCachePath` (model/ORT cache) but **no** store
 * location option: the vector-store backend location is C's internal concern
 * (currently an in-memory store), so we only pass `nodeCachePath`.
 */
export async function initializeEngine(): Promise<XbergEngine> {
  if (_engine !== null) return _engine;

  const cacheDir =
    process.env.XBERG_CACHE_DIR ?? join(homedir(), ".cache", "xberg");

  const injection = await createXbergRuntimeFactory({ nodeCachePath: cacheDir });
  _injection = injection;

  // Per Task 1 spec, engine construction uses default config.
  const { XbergEngine } = await import("@xberg-io/xberg-wasm");
  _engine = new XbergEngine({}, injection);

  return _engine;
}

/** Return the initialized singleton engine, or throw if not yet initialized. */
export function getEngine(): XbergEngine {
  if (_engine === null) {
    throw new Error("Engine not initialized. Call initializeEngine() first.");
  }
  return _engine;
}

/** Return the injected runtime descriptor (embedder/store/ner/ocr), or throw if not yet initialized. */
export function getRuntime(): InjectionDescriptor {
  if (_injection === null) {
    throw new Error("Engine not initialized. Call initializeEngine() first.");
  }
  return _injection;
}
