import { getBinding } from "../core/binding.js";
import type { EmbeddingConfig } from "../types.js";

/**
 * Embedding preset configuration.
 *
 * Contains all settings for a specific embedding model preset.
 */
export interface EmbeddingPreset {
	/** Name of the preset (e.g., "fast", "balanced", "quality", "multilingual") */
	name: string;
	/** Recommended chunk size in characters */
	chunkSize: number;
	/** Recommended overlap in characters */
	overlap: number;
	/** Model identifier (e.g., "AllMiniLML6V2Q", "BGEBaseENV15") */
	modelName: string;
	/** Embedding vector dimensions */
	dimensions: number;
	/** Human-readable description of the preset */
	description: string;
}

/**
 * Get all available embedding presets.
 *
 * Returns an array of names of all available embedding model presets.
 *
 * @returns Array of preset names (e.g., ["fast", "balanced", "quality", "multilingual"])
 *
 * @example
 * ```typescript
 * import { listEmbeddingPresets } from '@kreuzberg/node';
 *
 * const presets = listEmbeddingPresets();
 * console.log('Available presets:', presets);
 * ```
 */
export function listEmbeddingPresets(): string[] {
	const binding = getBinding();
	return binding.listEmbeddingPresets();
}

/**
 * Get embedding preset configuration by name.
 *
 * Retrieves the configuration for a specific embedding model preset.
 * Returns null if the preset doesn't exist.
 *
 * @param name - Name of the preset (e.g., "balanced", "fast", "quality")
 * @returns EmbeddingPreset configuration if found, null otherwise
 *
 * @example
 * ```typescript
 * import { getEmbeddingPreset } from '@kreuzberg/node';
 *
 * const preset = getEmbeddingPreset('balanced');
 * if (preset) {
 *   console.log(`Model: ${preset.modelName}, Dims: ${preset.dimensions}`);
 *   // Model: BGEBaseENV15, Dims: 768
 * }
 * ```
 */
export function getEmbeddingPreset(name: string): EmbeddingPreset | null {
	const binding = getBinding();
	const result = binding.getEmbeddingPreset(name);
	return result as unknown as EmbeddingPreset | null;
}

/**
 * Set embedding preset configuration.
 *
 * Note: The native binding does not expose a setEmbeddingPreset method.
 * Embedding presets are typically configured at the Rust level or through
 * configuration. This function is provided for API consistency.
 *
 * @param _name - Name of the preset (not used - for API consistency)
 * @param _preset - Preset configuration (not used - for API consistency)
 * @throws {Error} Not implemented - embedding presets cannot be set from TypeScript
 *
 * @example
 * ```typescript
 * // Embedding presets are typically defined in Rust or configuration
 * // This function is not available in the current API
 * ```
 */
export function setEmbeddingPreset(_name: string, _preset: EmbeddingPreset): void {
	throw new Error("setEmbeddingPreset is not available. Embedding presets must be configured at the Rust level.");
}

/**
 * Generate vector embeddings for a list of texts (synchronous).
 *
 * Requires the `embeddings` feature to be enabled (ONNX Runtime must be available).
 * Returns one float32 array per input text. An empty input returns an empty array.
 *
 * @param texts - Array of strings to embed
 * @param config - Optional embedding configuration (model preset, batch size, normalization)
 * @returns Array of float32 arrays (one embedding vector per input text)
 *
 * @throws {Error} If ONNX Runtime is not available or the model cannot be loaded
 *
 * @example
 * ```typescript
 * import { embedSync } from '@kreuzberg/node';
 *
 * const embeddings = embedSync(['Hello, world!'], { model: { type: 'preset', name: 'balanced' } });
 * console.log(embeddings.length); // 1
 * console.log(embeddings[0].length); // 768
 * ```
 */
export function embedSync(texts: string[], config?: EmbeddingConfig): number[][] {
	const binding = getBinding();
	return binding.embedSync(texts, (config ?? null) as Record<string, unknown> | null);
}

/**
 * Generate vector embeddings for a list of texts (asynchronous).
 *
 * Requires the `embeddings` feature to be enabled (ONNX Runtime must be available).
 * Returns one float32 array per input text. An empty input returns an empty array.
 *
 * @param texts - Array of strings to embed
 * @param config - Optional embedding configuration (model preset, batch size, normalization)
 * @returns Promise resolving to an array of float32 arrays (one embedding vector per input text)
 *
 * @throws {Error} If ONNX Runtime is not available or the model cannot be loaded
 *
 * @example
 * ```typescript
 * import { embed } from '@kreuzberg/node';
 *
 * const embeddings = await embed(['Hello, world!'], { model: { type: 'preset', name: 'balanced' } });
 * console.log(embeddings.length); // 1
 * console.log(embeddings[0].length); // 768
 * ```
 */
export async function embed(texts: string[], config?: EmbeddingConfig): Promise<number[][]> {
	const binding = getBinding();
	return binding.embed(texts, (config ?? null) as Record<string, unknown> | null);
}
