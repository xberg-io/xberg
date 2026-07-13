import { createEmbedder } from "./embedder.js";
import { createNer } from "./ner.js";
import { createOcr } from "./ocr.js";
import { configureTransformersEnvironment, defaultNodeCachePath } from "./runtime-env.js";

declare global {
	interface Window {
		ort?: { env: { wasm: { wasmPaths: string } } };
	}
}

interface ModelInfo {
	name: string;
	path: string;
}

/**
 * Pipeline handles that `warm()` can pre-download. Each maps to a factory in
 * this package whose `pipeline(...)` call performs the actual model fetch.
 */
type WarmHandle = "embedding" | "ner" | "ocr";

interface WarmOptions {
	/** Restrict warm-up to these model display names (see `MODELS`). */
	modelNames?: string[];
	/** Called once per pipeline handle before it is downloaded. */
	onProgress?: (phase: string) => void;
}

/** Maps legacy `MODELS` display names to the pipeline handles `warm()` knows. */
const MODEL_NAME_TO_HANDLE: Record<string, WarmHandle> = {
	"Embedder (minilm-l6-v2)": "embedding",
	"Embedder (all-MiniLM-L6-v2)": "embedding",
	"Embedder (bge-m3)": "embedding",
	"GLiNER2 NER": "ner",
	"BERT NER": "ner",
	"PP-OCRv6 OCR": "ocr",
};

const MODELS: ModelInfo[] = [
	{
		name: "Embedder (bge-m3)",
		path: "Xenova/bge-m3",
	},
	{
		name: "BERT NER",
		path: "Xenova/bert-base-NER",
	},
];

/**
 * Manages model cache in OPFS (browser, not yet implemented) or ~/.cache/xberg (Node).
 * Mirrors the MCP WarmupManager responsibilities.
 */
export class CacheManager {
	private cacheDir: string;

	constructor(cacheDir?: string) {
		this.cacheDir = cacheDir ?? this.defaultCacheDir();
	}

	private defaultCacheDir(): string {
		if (typeof window === "undefined") {
			// Node.js
			return defaultNodeCachePath();
		}
		// Browser: OPFS virtual path (actual I/O handled by wa-sqlite)
		return "/opfs/xberg";
	}

	async status(): Promise<{
		cached: string[];
		size: number;
	}> {
		const cached: string[] = [];
		let totalSize = 0;

		// `fs`/`path` are Node-only — imported lazily here (never reached in the
		// browser) rather than statically at module scope, so bundlers targeting
		// the browser (e.g. this package used from a Web Worker via Next.js/
		// webpack) never need to resolve them at all.
		const nodeFs = typeof window === "undefined" ? await import("fs") : null;
		const nodePath = typeof window === "undefined" ? await import("path") : null;

		for (const model of MODELS) {
			try {
				if (nodeFs && nodePath) {
					const modelPath = nodePath.join(this.cacheDir, model.path);
					if (nodeFs.existsSync(modelPath)) {
						const size = directorySize(nodeFs, nodePath, modelPath);
						if (size > 0) {
							cached.push(model.name);
							totalSize += size;
						}
					}
				} else {
					// Browser: check OPFS (simplified; actual check would use storage API)
					// For now, assume not cached in CI
				}
			} catch (err) {
				// File not found is expected; other errors should be logged
				if (err instanceof Error && "code" in err && err.code !== "ENOENT") {
					console.warn(`[cache] unexpected error checking ${model.name}:`, err);
				}
			}
		}

		return { cached, size: totalSize };
	}

	/**
	 * Pre-download and cache the model artifacts used by the SDK's embedder and
	 * NER pipelines so cold-start never blocks on a network fetch.
	 *
	 * The download is performed by routing `@huggingface/transformers` to this
	 * cache directory (`env.cacheDir`) and invoking the existing `createEmbedder`
	 * / `createNer` factories, whose `pipeline(...)` calls are what actually
	 * fetch and persist the model files. No model-download logic is reimplemented
	 * here.
	 *
	 * Accepts either a list of model display names (legacy form, see `MODELS`)
	 * or an options object carrying an `onProgress` callback. Returns the set of
	 * pipeline handles that succeeded / failed.
	 */
	async warm(modelNames?: string[]): Promise<{ success: string[]; failed: string[] }>;
	async warm(opts?: WarmOptions): Promise<{ success: string[]; failed: string[] }>;
	async warm(arg?: string[] | WarmOptions): Promise<{ success: string[]; failed: string[] }> {
		const opts: WarmOptions = Array.isArray(arg) ? { modelNames: arg } : (arg ?? {});

		const selected = opts.modelNames
			? opts.modelNames.map((name) => MODEL_NAME_TO_HANDLE[name]).filter((h): h is WarmHandle => h !== undefined)
			: (["embedding", "ner", "ocr"] as WarmHandle[]);

		// De-duplicate while preserving order: embedding first, then ner.
		const handles = ["embedding", "ner", "ocr"].filter((h) => selected.includes(h as WarmHandle)) as WarmHandle[];

		const success: string[] = [];
		const failed: string[] = [];

		// Route transformers.js downloads into this cache directory.
		configureTransformersEnvironment({ nodeCachePath: this.cacheDir });

		for (const handle of handles) {
			try {
				opts.onProgress?.(handle);
				// eslint-disable-next-line no-await-in-loop -- download sequentially to bound concurrency
				if (handle === "embedding") {
					// oxlint-disable-next-line no-await-in-loop -- model loads are intentionally memory-bounded
					await createEmbedder({ nodeCachePath: this.cacheDir });
				} else if (handle === "ner") {
					// oxlint-disable-next-line no-await-in-loop -- model loads are intentionally memory-bounded
					const ner = await createNer({ nodeCachePath: this.cacheDir });
					if (!ner) throw new Error("NER model initialization returned null");
				} else {
					// oxlint-disable-next-line no-await-in-loop -- model loads are intentionally memory-bounded
					const ocr = await createOcr();
					if (!ocr) throw new Error("OCR model initialization returned null");
				}
				success.push(handle);
			} catch (err) {
				console.error(`[cache] warm failed for ${handle}:`, err);
				failed.push(handle);
			}
		}

		return { success, failed };
	}

	/**
	 * Set ONNX Runtime wasm binary paths to self-hosted location (no CDN).
	 */
	setWasmPaths(wasmDir: string): void {
		try {
			if (typeof window !== "undefined" && "ort" in window && window.ort) {
				window.ort.env.wasm.wasmPaths = wasmDir;
				console.debug(`[cache] ORT wasm paths set to ${wasmDir}`);
			} else {
				console.debug(`[cache] window.ort not found; setWasmPaths is a no-op`);
			}
		} catch (err) {
			console.warn(`[cache] failed to set ORT wasm paths:`, err);
		}
	}
}

function directorySize(nodeFs: typeof import("fs"), nodePath: typeof import("path"), directory: string): number {
	return nodeFs.readdirSync(directory, { withFileTypes: true }).reduce((total, entry) => {
		const child = nodePath.join(directory, entry.name);
		return (
			total + (entry.isDirectory() ? directorySize(nodeFs, nodePath, child) : nodeFs.statSync(child).size)
		);
	}, 0);
}
