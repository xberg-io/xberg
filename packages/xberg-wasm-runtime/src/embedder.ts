import { pipeline, env } from "@huggingface/transformers";
import type { CacheConfig, EmbedderInterface } from "./types.js";
import { selectModelBackend, createPipelineWithFallback } from "./backend.js";
import { configureTransformersEnvironment } from "./runtime-env.js";

// Suppress remote-model fetching in CI once models are already cached locally.
if (typeof process !== "undefined" && process.env.CI) {
	env.allowLocalModels = true;
}

const DEFAULT_MODEL = "Xenova/bge-m3";
const DEFAULT_BATCH_SIZE = 32;
const MAX_CACHE_ENTRIES = 1_024;

async function sha256Hex(input: string): Promise<string> {
	const bytes = new TextEncoder().encode(input);
	const digest = await crypto.subtle.digest("SHA-256", bytes);
	return Array.from(new Uint8Array(digest))
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("");
}

/**
 * Create an embedder using transformers.js v3 + ONNX Runtime Web.
 * Vectors are L2-normalized before return (unit-length, matching rag-embeddings rule).
 * WebGPU is used when available; otherwise quantized WASM-CPU is selected.
 */
export async function createEmbedder(config?: CacheConfig): Promise<EmbedderInterface> {
	const modelId = config?.models?.embedder ?? DEFAULT_MODEL;
	configureTransformersEnvironment(config);

	const backend = await selectModelBackend(config);
	console.debug(`[embedder] device=${backend.device} dtype=${backend.dtype} model=${modelId}`);
	const extractor = await createPipelineWithFallback(
		(b) => pipeline("feature-extraction", modelId, b),
		backend,
		"embedder",
		modelId,
	);

	const cache = new Map<string, Float32Array>();

	async function embed(texts: string[]): Promise<Float32Array[]> {
		if (texts.length === 0) return [];

		const hashes = await Promise.all(texts.map((t) => sha256Hex(`${modelId}:${t}`)));
		const results: (Float32Array | undefined)[] = texts.map((_, i) => {
			const h = hashes[i];
			if (h === undefined) return undefined;
			const cached = cache.get(h);
			if (cached) {
				cache.delete(h);
				cache.set(h, cached);
			}
			return cached;
		});

		const uncachedIndices = results.map((r, i) => (r === undefined ? i : -1)).filter((i) => i !== -1);
		const uncachedTexts = uncachedIndices.map((i) => texts[i]).filter((t): t is string => t !== undefined);

		for (let i = 0; i < uncachedTexts.length; i += DEFAULT_BATCH_SIZE) {
			const batch = uncachedTexts.slice(i, Math.min(i + DEFAULT_BATCH_SIZE, uncachedTexts.length));
			const batchIndices = uncachedIndices.slice(i, Math.min(i + DEFAULT_BATCH_SIZE, uncachedIndices.length));

			// eslint-disable-next-line no-await-in-loop -- one batch at a time; bounds peak memory and preserves order
			const output = await extractor(batch, { pooling: "mean", normalize: false });

			const [batchSize, hiddenSize] = output.dims;
			if (batchSize === undefined || hiddenSize === undefined) {
				throw new Error(`Unexpected feature-extraction output shape: [${output.dims.join(", ")}]`);
			}
			const flat = Float32Array.from(output.data as ArrayLike<number>);

			for (let row = 0; row < batchSize; row++) {
				const start = row * hiddenSize;
				const vec = l2Normalize(flat.subarray(start, start + hiddenSize));
				const originalIndex = batchIndices[row];
				if (originalIndex === undefined) continue;
				results[originalIndex] = vec;
				const h = hashes[originalIndex];
				if (h !== undefined) {
					cache.set(h, vec);
					if (cache.size > MAX_CACHE_ENTRIES) {
						const oldestKey = cache.keys().next().value;
						if (oldestKey !== undefined) cache.delete(oldestKey);
					}
				}
			}
		}

		return results as Float32Array[];
	}

	return { embed };
}

/**
 * L2-normalize a vector to unit length.
 */
function l2Normalize(vec: Float32Array): Float32Array {
	let sumOfSquares = 0;
	for (const v of vec) {
		sumOfSquares += v * v;
	}
	const magnitude = Math.sqrt(sumOfSquares);
	if (magnitude === 0) return new Float32Array(vec);

	return Float32Array.from(vec, (v) => v / magnitude);
}
