import { getBinding } from "../core/binding.js";
import type { Chunk, ExtractedImage, ExtractionResult, PostProcessorProtocol, Table } from "../types.js";

/**
 * Register a custom post-processor.
 *
 * Post-processors allow you to hook into the extraction pipeline and transform
 * the extraction results. They run after the core extraction is complete.
 *
 * Post-processors are async and can modify extraction results before they are
 * returned to the caller.
 *
 * @param processor - Post-processor implementing PostProcessorProtocol
 *
 * @example
 * ```typescript
 * import { registerPostProcessor, extractFile } from '@kreuzberg/node';
 *
 * class CustomProcessor {
 *   name() {
 *     return 'custom_processor';
 *   }
 *   processingStage() {
 *     return 'post';
 *   }
 *   async process(result) {
 *     // Add custom metadata
 *     result.metadata.customField = 'custom_value';
 *     return result;
 *   }
 * }
 *
 * // Use async extraction (required for custom processors)
 * const result = await extractFile('document.pdf');
 * console.log(result.metadata.customField); // 'custom_value'
 * ```
 */
export function registerPostProcessor(processor: PostProcessorProtocol): void {
	const binding = getBinding();

	const wrappedProcessor = {
		name: typeof processor.name === "function" ? processor.name() : processor.name,
		processingStage:
			typeof processor.processingStage === "function" ? processor.processingStage() : processor.processingStage,
		async process(...args: unknown[]): Promise<string> {
			const wrappedValue = args[0] as unknown[];
			const jsonString = wrappedValue[0] as string;

			const wireResult = JSON.parse(jsonString) as {
				content: string;
				mime_type: string;
				metadata: string | Record<string, unknown>;
				tables?: unknown[];
				detected_languages?: string[];
				chunks?: unknown[];
				images?: unknown[];
			};

			const result: ExtractionResult = {
				content: wireResult.content,
				mimeType: wireResult.mime_type,
				metadata: typeof wireResult.metadata === "string" ? JSON.parse(wireResult.metadata) : wireResult.metadata,
				tables: (wireResult.tables || []) as Table[],
				detectedLanguages: wireResult.detected_languages ?? null,
				chunks: (wireResult.chunks as Chunk[] | null | undefined) ?? null,
				images: (wireResult.images as ExtractedImage[] | null | undefined) ?? null,
			};

			const updated = await processor.process(result);

			const wireUpdated = {
				content: updated.content,
				mime_type: updated.mimeType,
				metadata: updated.metadata,
				tables: updated.tables,
				detected_languages: updated.detectedLanguages,
				chunks: updated.chunks,
				images: updated.images,
			};

			return JSON.stringify(wireUpdated);
		},
	};

	Object.defineProperty(wrappedProcessor, "__original", {
		value: processor,
		enumerable: false,
	});

	const stage = processor.processingStage?.() ?? "middle";
	Object.defineProperty(wrappedProcessor, "__stage", {
		value: stage,
		enumerable: false,
	});

	binding.registerPostProcessor(wrappedProcessor);
}

/**
 * Unregister a postprocessor by name.
 *
 * Removes a previously registered postprocessor from the registry.
 * If the processor doesn't exist, this is a no-op (does not throw).
 *
 * @param name - Name of the processor to unregister (case-sensitive)
 *
 * @example
 * ```typescript
 * import { unregisterPostProcessor } from '@kreuzberg/node';
 *
 * unregisterPostProcessor('my_processor');
 * ```
 */
export function unregisterPostProcessor(name: string): void {
	const binding = getBinding();
	binding.unregisterPostProcessor(name);
}

/**
 * Clear all registered postprocessors.
 *
 * Removes all postprocessors from the registry. Useful for test cleanup or resetting state.
 * If no postprocessors are registered, this is a no-op.
 *
 * @example
 * ```typescript
 * import { clearPostProcessors } from '@kreuzberg/node';
 *
 * clearPostProcessors();
 * ```
 */
export function clearPostProcessors(): void {
	const binding = getBinding();
	binding.clearPostProcessors();
}

/**
 * List all registered post-processors.
 *
 * Returns the names of all currently registered post-processors (both built-in and custom).
 *
 * @returns Array of post-processor names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listPostProcessors } from '@kreuzberg/node';
 *
 * const names = listPostProcessors();
 * console.log('Registered post-processors:', names);
 * ```
 */
export function listPostProcessors(): string[] {
	const binding = getBinding();
	return binding.listPostProcessors();
}

/**
 * Get a registered post-processor by name.
 *
 * Retrieves information about a specific post-processor from the registry.
 *
 * @param name - Name of the post-processor to retrieve
 * @returns The post-processor if found, null otherwise
 *
 * @example
 * ```typescript
 * import { getPostProcessor } from '@kreuzberg/node';
 *
 * const processor = getPostProcessor('my_processor');
 * if (processor) {
 *   console.log('Processor found:', processor.name);
 * }
 * ```
 */
export function getPostProcessor(name: string): unknown {
	// Note: This function is not directly exposed by the native binding
	// It's a helper function that uses listPostProcessors to check if a processor exists
	const processors = listPostProcessors();
	return processors.includes(name) ? { name } : null;
}
