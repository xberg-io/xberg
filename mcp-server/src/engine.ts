import type { XbergEngine } from "@xberg-io/xberg-wasm";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import type { InjectionDescriptor } from "xberg-wasm-runtime";
import { getCacheDir } from "./paths.js";

let _engine: XbergEngine | null = null;
let _injection: InjectionDescriptor | null = null;
let _initPromise: Promise<XbergEngine> | null = null;

/**
 * Build the `XbergEngine` (B) once, wiring it to C's shared runtime factory.
 *
 * C's single public entry point `createXbergRuntimeFactory` constructs and
 * validates the whole injection descriptor (`{ embedder, store, ner?, ocr? }`)
 * that B's constructor consumes — we do not touch the per-capability factories
 * directly.
 *
 * `CacheConfig` exposes `nodeCachePath` (model/ORT cache) but no explicit
 * store location — the vector-store backend location is C's internal
 * concern. C's default Node store persists to `<nodeCachePath>/store.sqlite3`
 * (see `store-node.ts`'s `createNodeVectorStore`), so ingested documents
 * survive process restarts; pass `nodeStorePath` explicitly to relocate it.
 */
export function initializeEngine(): Promise<XbergEngine> {
  // Guard the async startup with a cached promise so concurrent callers share a
  // single initialization instead of each racing past the `_engine === null`
  // check and constructing a duplicate engine.
  if (_initPromise !== null) return _initPromise;

  const init = (async () => {
    if (_engine !== null) return _engine;

    const cacheDir = getCacheDir();

    // `nerBackend: "candle"` matters here specifically because of how
    // `resolve_ingest_ner` (crates/xberg-wasm/src/bridge/ner.rs) picks a
    // backend for `ingest()`'s mandatory PII redaction step: an injected JS
    // NER bridge (the default transformers.js pipeline this factory would
    // otherwise create) always takes priority over the in-binary Candle
    // GLiNER2 backend. Without this flag, `ingest()` silently redacted with
    // the weak generic bert-base-NER model instead of the real
    // privacy-tuned GLiNER2 model. `xberg-wasm-runtime`'s
    // `initCandleNerBackend` (invoked internally when this flag is set)
    // downloads/caches the model under `<nodeCachePath>/candle-ner/` and
    // omits `ner` from the returned descriptor on success, letting
    // `resolve_ingest_ner` fall through to Candle; on failure it falls back
    // to injecting the transformers.js NER instead, non-fatally.
    const injection = await createXbergRuntimeFactory({ nodeCachePath: cacheDir, nerBackend: "candle" });
    _injection = injection;

    // Per Task 1 spec, engine construction uses default config.
    const { XbergEngine } = await import("@xberg-io/xberg-wasm");
    _engine = new XbergEngine({}, injection);

    return _engine;
  })();

  // Reset the cached promise on failure so a transient startup error can be
  // retried on the next call. Guard against clearing a newer initialization
  // attempt that may have superseded this one while the async work was pending.
  const guarded = init.catch((err) => {
    if (_initPromise === guarded) _initPromise = null;
    throw err;
  });
  _initPromise = guarded;
  return guarded;
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
