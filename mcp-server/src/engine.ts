import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
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

    const injection = await createXbergRuntimeFactory({ nodeCachePath: cacheDir });
    _injection = injection;

    // Per Task 1 spec, engine construction uses default config.
    const { XbergEngine } = await import("@xberg-io/xberg-wasm");
    _engine = new XbergEngine({}, injection);

    // `ingest()` requires the in-binary Candle NER backend for its mandatory
    // PII+NER redaction step. The injected NER bridge only powers
    // `Engine.ner()`, not `ingest()`, so load the Candle model here. Model
    // loading is best-effort: a failure (missing model files, network error)
    // is logged but does not abort startup — `ingest()` will surface a clear
    // error at call time instead.
    await initCandleNer(_engine, cacheDir);

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

/**
 * Load the in-binary Candle NER model into the engine via `initCandleNer`.
 *
 * The model ships as three files — `model.safetensors`, `tokenizer.json`,
 * `encoder_config.json`. We look for them under `<cacheDir>/candle-ner/`; if
 * present, they are read and passed to the engine. A Hugging Face repo can be
 * supplied via `XBERG_CANDLE_NER_REPO` to fetch the files on first run. Any
 * failure is non-fatal: we log and return so startup continues.
 */
async function initCandleNer(engine: XbergEngine, cacheDir: string): Promise<void> {
  try {
    const repo = process.env.XBERG_CANDLE_NER_REPO;
    const dir = join(cacheDir, "candle-ner");
    const safetensors = await resolveModelFile(dir, "model.safetensors", repo);
    const tokenizerJson = await resolveModelFile(dir, "tokenizer.json", repo);
    const encoderConfig = await resolveModelFile(dir, "encoder_config.json", repo);
    if (safetensors && tokenizerJson && encoderConfig) {
      engine.initCandleNer(safetensors, tokenizerJson, encoderConfig);
    } else {
      console.warn(
        "[engine] Candle NER model files not found; ingest() will fail until initCandleNer is called with model bytes.",
      );
    }
  } catch (err) {
    console.warn(`[engine] Candle NER not initialized: ${String(err)}. ingest() will fail until initCandleNer is called.`);
  }
}

/**
 * Resolve a model file from `dir` if present, otherwise download it from
 * `<repo>/resolve/main/<name>` into `dir` when `repo` is set. Returns `null`
 * if the file is neither present locally nor fetchable.
 */
async function resolveModelFile(
  dir: string,
  name: string,
  repo: string | undefined,
): Promise<Uint8Array | null> {
  const localPath = join(dir, name);
  if (existsSync(localPath)) {
    return new Uint8Array(readFileSync(localPath));
  }
  if (!repo) return null;
  const url = `https://huggingface.co/${repo}/resolve/main/${name}`;
  const res = await fetch(url);
  if (!res.ok || !res.body) return null;
  const buf = new Uint8Array(await res.arrayBuffer());
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  writeFileSync(localPath, buf);
  return buf;
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
