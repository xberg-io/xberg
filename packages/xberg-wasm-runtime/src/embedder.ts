import { pipeline, env } from "@huggingface/transformers";
import type { CacheConfig, EmbedderInterface } from "./types";

// Suppress remote-model fetching in CI once models are already cached locally.
if (typeof process !== "undefined" && process.env.CI) {
  env.allowLocalModels = true;
}

const DEFAULT_MODEL = "Xenova/all-MiniLM-L6-v2";
const DEFAULT_BATCH_SIZE = 32;

/**
 * Create an embedder using transformers.js v3 + ONNX Runtime Web.
 * Vectors are L2-normalized before return (unit-length, matching rag-embeddings rule).
 * WebGPU is used when available; silently falls back to WASM-CPU.
 */
export async function createEmbedder(
  config?: CacheConfig
): Promise<EmbedderInterface> {
  const modelId = config?.models?.embedder ?? DEFAULT_MODEL;

  // Initialize the feature extraction pipeline (embeddings).
  const extractor = await pipeline("feature-extraction", modelId);

  async function embed(texts: string[]): Promise<Float32Array[]> {
    if (texts.length === 0) return [];

    const results: Float32Array[] = [];

    // Process in batches to manage memory. Batches are awaited sequentially
    // (not Promise.all) so at most one batch's tensor output is resident in
    // memory at a time, and so `results` preserves input order — pushing
    // from concurrently-resolving batches would make output order depend on
    // resolution timing rather than input position.
    for (let i = 0; i < texts.length; i += DEFAULT_BATCH_SIZE) {
      const batch = texts.slice(
        i,
        Math.min(i + DEFAULT_BATCH_SIZE, texts.length)
      );

      // Mean-pool token embeddings into a single sentence embedding per input.
      // We normalize ourselves below rather than relying on the pipeline's
      // built-in `normalize` option, to keep the normalization logic explicit
      // and unit-tested in this module.
      // eslint-disable-next-line no-await-in-loop -- intentional: bounds
      // peak memory to one batch and preserves output ordering (see comment
      // above the loop).
      const output = await extractor(batch, {
        pooling: "mean",
        normalize: false,
      });

      // `output` is a Tensor with shape [batch.length, hiddenSize] and a flat
      // `.data` array. Slice out each row before normalizing.
      const [batchSize, hiddenSize] = output.dims;
      if (batchSize === undefined || hiddenSize === undefined) {
        throw new Error(
          `Unexpected feature-extraction output shape: [${output.dims.join(", ")}]`
        );
      }
      const flat = Float32Array.from(output.data as ArrayLike<number>);

      for (let row = 0; row < batchSize; row++) {
        const start = row * hiddenSize;
        const vec = flat.subarray(start, start + hiddenSize);
        results.push(l2Normalize(vec));
      }
    }

    return results;
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
