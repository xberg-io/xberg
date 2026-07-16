/**
 * Wiring for the in-binary Candle GLiNER2 NER backend
 * (`crates/xberg-gliner-candle`, exposed via `initCandleNer` in
 * `crates/xberg-wasm/src/bridge/ner.rs`). Unlike `ner.ts`'s transformers.js
 * `bert-base-NER` pipeline (fixed PER/ORG/LOC/MISC labels), this is real
 * GLiNER2: a schema-prompt zero-shot NER model that accepts arbitrary
 * label strings (email/phone/person/location/... — see
 * `crates/xberg/src/types/entity.rs`'s `EntityCategory`) at call time.
 *
 * `initCandleNer` is a free function on the wasm module (not a method on
 * `XbergEngine`) that populates a `thread_local!` inside the compiled
 * binary -- once called, `crates/xberg-wasm/src/bridge/ner.rs::resolve_ner`
 * uses it automatically as the fallback path whenever `XbergEngine` is NOT
 * given an injected `ner` object. That means the caller (`factory.ts`) must
 * omit `ner` from the returned `InjectionDescriptor` for this to actually
 * take effect, not just call `initCandleNer` and also inject the JS NER.
 *
 * Works in both browser and Node: `@xberg-io/xberg-wasm` resolves to
 * `pkg/nodejs` by default and to `pkg/web` only via the browser-side
 * bundler alias xberg-web-ui's next.config.js sets up, and
 * `initCandleNer`/`init` exist on both targets. This module previously
 * refused to run outside `typeof window !== "undefined"` on the assumption
 * that mcp-server would instead use a native ONNX Runtime GLiNER
 * implementation -- that native path (crates/xberg-node) was never wired up
 * end-to-end, while this WASM path works unmodified in Node (confirmed live
 * against mcp-server): `fetch` is a Node 18+ global and `fetchWithCache`
 * already degrades to a plain fetch when the browser Cache Storage API
 * (`caches`) is absent, which is exactly the Node case.
 */
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { CacheConfig } from "./types.js";

// fastino/gliner2-privacy-filter-PII-multi: the pinned Candle-compatible
// GLiNER2 PII model (~1.24GB total across the three files below). Confirmed
// via the HF Hub API to ship model.safetensors (single-file, not sharded --
// required, see model.rs's `from_buffered_safetensors`), tokenizer.json, and
// encoder_config/config.json (a DeBERTa-v2-style config, deserialized
// directly as `candle_transformers::models::debertav2::Config`).
const DEFAULT_CANDLE_NER_BASE_URL = "https://huggingface.co/fastino/gliner2-privacy-filter-PII-multi/resolve/main/";
const CANDLE_NER_CACHE_NAME = "xberg-candle-ner-v1";

interface CandleNerModelBytes {
	safetensors: Uint8Array;
	tokenizerJson: Uint8Array;
	encoderConfigJson: Uint8Array;
}

/**
 * Fetch `url`, persisting the response across restarts when a cache backend
 * is available: the browser Cache Storage API in the browser, or a plain
 * file under `nodeCacheDir` in Node (mirrors mcp-server's own
 * disk-persisted model cache convention -- this is a genuinely large,
 * ~1.24GB total download, so avoiding a re-fetch on every process restart
 * matters). Falls back to a plain fetch if no cache backend is available or
 * a cache write fails (matches the resilience pattern transformers.js
 * itself uses for its own model cache -- see the "Unable to add response to
 * browser cache" warning path in `@huggingface/transformers`' hub.js, which
 * never treats a cache-write failure as fatal).
 */
async function fetchWithCache(url: string, nodeCacheDir: string | undefined, fileName: string): Promise<Uint8Array> {
	if (nodeCacheDir) {
		const localPath = join(nodeCacheDir, fileName);
		// A zero-byte file means a previous download was interrupted --
		// treat it as absent so it retries, rather than "successfully"
		// loading an empty model file.
		if (existsSync(localPath) && statSync(localPath).size > 0) {
			return new Uint8Array(readFileSync(localPath));
		}
	}

	const cacheApi = typeof caches !== "undefined" ? caches : undefined;
	if (cacheApi) {
		try {
			const cache = await cacheApi.open(CANDLE_NER_CACHE_NAME);
			const cached = await cache.match(url);
			if (cached) {
				return new Uint8Array(await cached.arrayBuffer());
			}
		} catch (err) {
			console.warn(`[candle-ner] cache read failed for ${url}, fetching directly:`, err);
		}
	}

	const response = await fetch(url);
	if (!response.ok) {
		throw new Error(`[candle-ner] fetch failed for ${url}: ${response.status} ${response.statusText}`);
	}

	if (cacheApi) {
		try {
			const cache = await cacheApi.open(CANDLE_NER_CACHE_NAME);
			await cache.put(url, response.clone());
		} catch (err) {
			console.warn(`[candle-ner] unable to cache response for ${url}:`, err);
		}
	}

	const bytes = new Uint8Array(await response.arrayBuffer());

	if (nodeCacheDir) {
		try {
			const localPath = join(nodeCacheDir, fileName);
			mkdirSync(dirname(localPath), { recursive: true });
			writeFileSync(localPath, bytes);
		} catch (err) {
			console.warn(`[candle-ner] unable to persist ${fileName} to node cache:`, err);
		}
	}

	return bytes;
}

/**
 * Download the three files `CandleBackend::from_bytes` requires. Fetched in
 * parallel; the safetensors file dominates total size (~1.2GB of the
 * ~1.24GB total), so this is a genuinely large, multi-minute download on a
 * cold cache -- callers should surface progress/expectations to the user
 * rather than assume this resolves quickly.
 */
async function downloadCandleNerModel(baseUrl: string, nodeCacheDir: string | undefined): Promise<CandleNerModelBytes> {
	const [safetensors, tokenizerJson, encoderConfigJson] = await Promise.all([
		fetchWithCache(new URL("model.safetensors", baseUrl).href, nodeCacheDir, "model.safetensors"),
		fetchWithCache(new URL("tokenizer.json", baseUrl).href, nodeCacheDir, "tokenizer.json"),
		fetchWithCache(new URL("encoder_config/config.json", baseUrl).href, nodeCacheDir, join("encoder_config", "config.json")),
	]);
	return { safetensors, tokenizerJson, encoderConfigJson };
}

/**
 * Download the pinned GLiNER2 PII model and initialize the in-binary Candle
 * NER backend via `@xberg-io/xberg-wasm`'s `initCandleNer`. Works in both
 * browser and Node -- see this module's doc comment for why the Node path
 * is safe. In Node, `config?.nodeCachePath` (if set) doubles as a
 * `<nodeCachePath>/candle-ner/` disk cache for the three model files, so
 * the ~1.24GB download only happens once across process restarts.
 *
 * Idempotent-safe to call multiple times: `initCandleNer` replaces the
 * previously-loaded model rather than erroring, and the underlying wasm
 * `init()` short-circuits once the module is already instantiated.
 */
export async function initCandleNerBackend(config?: CacheConfig): Promise<void> {
	const baseUrl = config?.candleNerModelUrl ?? DEFAULT_CANDLE_NER_BASE_URL;
	const nodeCacheDir = config?.nodeCachePath ? join(config.nodeCachePath, "candle-ner") : undefined;
	console.debug(`[candle-ner] downloading GLiNER2 PII model from ${baseUrl}`);

	const [wasmModule, bytes] = await Promise.all([
		import("@xberg-io/xberg-wasm"),
		downloadCandleNerModel(baseUrl, nodeCacheDir),
	]);

	// Idempotent (see xberg_wasm.js's __wbg_init: `if (wasm !== undefined) return wasm;`).
	// Safe even if the caller already called `init()` before constructing
	// XbergEngine -- initCandleNer operates on the same module-global wasm
	// instance either way, so ordering relative to XbergEngine construction
	// does not matter, only that the module is instantiated before this call.
	//
	// `@xberg-io/xberg-wasm` resolves to pkg/nodejs's types by default (no
	// `default` export -- the Node wasm-bindgen target auto-initializes
	// synchronously), even though the browser bundler alias substitutes
	// pkg/web at build time (which DOES need this async init). Read `default`
	// dynamically rather than statically so this typechecks against either
	// target; harmless no-op on a target that doesn't need it.
	const init = (wasmModule as unknown as { default?: () => Promise<unknown> }).default;
	if (typeof init === "function") {
		await init();
	}

	wasmModule.initCandleNer(bytes.safetensors, bytes.tokenizerJson, bytes.encoderConfigJson);
	console.debug("[candle-ner] GLiNER2 PII model loaded");
}
