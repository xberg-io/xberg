import { createEmbedder } from "./embedder.js";
import { createVectorStore } from "./store.js";
import { createNer } from "./ner.js";
import { createOcr } from "./ocr.js";
import { CacheManager } from "./cache.js";
import { validateInjectionDescriptor } from "./validation.js";
import type { CacheConfig, EmbedderInterface, InjectionDescriptor, VectorStoreInterface } from "./types.js";

/**
 * Create a complete injection descriptor for the wasm engine.
 * This is the main entry point for integrating xberg-wasm-runtime into a frontend.
 *
 * @param config Optional cache and model configuration
 * @returns A fully-constructed InjectionDescriptor ready for XbergEngine constructor
 * @throws If required components (embedder, store) fail to initialize
 */
export async function createXbergRuntimeFactory(config?: CacheConfig): Promise<InjectionDescriptor> {
	// Initialize cache manager (handles model warmup and ORT wasm paths)
	const cache = new CacheManager(config?.nodeCachePath);
	if (config?.wasmPaths) {
		cache.setWasmPaths(config.wasmPaths);
	}

	// Create required components
	let embedder: EmbedderInterface;
	let store: VectorStoreInterface;

	try {
		embedder = await createEmbedder(config);
	} catch (err) {
		throw new Error(`[factory] embedder initialization failed: ${err}`, { cause: err });
	}

	try {
		store = await createVectorStore(config);
	} catch (err) {
		throw new Error(`[factory] vector store initialization failed: ${err}`, { cause: err });
	}

	// Create optional components (null if unavailable)
	const ner = await createNer(config).catch((e) => {
		console.warn("[factory] NER initialization failed, using fallback:", e);
		return null;
	});

	const ocr = await createOcr(config).catch((e) => {
		console.warn("[factory] OCR initialization failed, using fallback:", e);
		return null;
	});

	// Build the descriptor
	const descriptor: InjectionDescriptor = {
		embedder,
		store,
		...(ner && { ner }),
		...(ocr && { ocr }),
	};

	// Validate the descriptor before returning
	const validation = validateInjectionDescriptor(descriptor);
	if (!validation.valid) {
		throw new Error(`[factory] validation failed: ${validation.error}`);
	}

	console.debug(
		"[factory] injection descriptor created",
		ner ? "(with NER)" : "(no NER)",
		ocr ? "(with OCR)" : "(no OCR)",
	);

	return descriptor;
}
