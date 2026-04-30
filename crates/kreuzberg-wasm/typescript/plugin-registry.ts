/**
 * Plugin Registry Module
 *
 * This module manages registrations and execution of post-processors and validators
 * for document extraction pipelines.
 *
 * # Thread Safety
 * All registrations are stored in Maps and are single-threaded safe for WASM environments.
 *
 * # Global Callback Functions
 * The WASM module can invoke processing via global callback functions:
 * - `__kreuzberg_execute_post_processor`: Execute a registered post-processor
 * - `__kreuzberg_execute_validator`: Execute a registered validator
 */

import type { ExtractionResult } from "./types.js";

/**
 * Post-processor plugin interface
 *
 * A post-processor modifies extraction results after extraction completes.
 */
export interface PostProcessor {
	/**
	 * Get the processor name (must be non-empty string)
	 */
	name(): string;

	/**
	 * Get the processing stage (optional, defaults to "middle")
	 * - "early": Process early in the pipeline
	 * - "middle": Process in the middle of the pipeline
	 * - "late": Process late in the pipeline
	 */
	stage?(): "early" | "middle" | "late";

	/**
	 * Process an extraction result
	 * Can be sync or async
	 */
	process(result: ExtractionResult): ExtractionResult | Promise<ExtractionResult>;

	/**
	 * Shutdown the processor (optional)
	 */
	shutdown?(): void | Promise<void>;
}

/**
 * Validator plugin interface
 *
 * A validator checks extraction results for correctness
 */
export interface Validator {
	/**
	 * Get the validator name (must be non-empty string)
	 */
	name(): string;

	/**
	 * Get the validation priority (optional, defaults to 50)
	 * Higher numbers = higher priority (execute first)
	 */
	priority?(): number;

	/**
	 * Validate an extraction result
	 * Can be sync or async
	 */
	validate(
		result: ExtractionResult,
	): { valid: boolean; errors: string[] } | Promise<{ valid: boolean; errors: string[] }>;

	/**
	 * Shutdown the validator (optional)
	 */
	shutdown?(): void | Promise<void>;
}

/** Map of post-processor name -> processor instance */
const postProcessors = new Map<string, PostProcessor>();

/** Map of validator name -> validator instance */
const validators = new Map<string, Validator>();

// ============================================================================
// Post-Processor Registry Functions
// ============================================================================

/**
 * Validate a post-processor object
 *
 * @throws {Error} If the processor doesn't implement required methods
 */
function validatePostProcessor(processor: unknown): processor is PostProcessor {
	if (processor === null || processor === undefined) {
		throw new Error("Post-processor cannot be null or undefined");
	}

	const obj = processor as Record<string, unknown>;

	if (typeof obj.name !== "function") {
		throw new Error("Post-processor must implement name() method");
	}

	if (typeof obj.process !== "function") {
		throw new Error("Post-processor must implement process() method");
	}

	const name = obj.name();
	if (typeof name !== "string" || name.trim() === "") {
		throw new Error("Post-processor name must be a non-empty string");
	}

	return true;
}

/**
 * Register a post-processor plugin
 *
 * @param processor - The post-processor to register
 * @throws {Error} If the processor is invalid or missing required methods
 *
 * @example
 * ```typescript
 * const processor = {
 *   name: () => "my-processor",
 *   stage: () => "middle",
 *   process: async (result) => {
 *     result.content = result.content.toUpperCase();
 *     return result;
 *   }
 * };
 * registerPostProcessor(processor);
 * ```
 */
export function registerPostProcessor(processor: PostProcessor): void {
	validatePostProcessor(processor);

	const name = processor.name();

	if (postProcessors.has(name)) {
		console.warn(`Post-processor "${name}" already registered, overwriting with new implementation`);
	}

	postProcessors.set(name, processor);
}

/**
 * Get a registered post-processor by name
 *
 * @param name - The processor name
 * @returns The processor, or undefined if not found
 *
 * @example
 * ```typescript
 * const processor = getPostProcessor("my-processor");
 * if (processor) {
 *   console.log("Found processor:", processor.name());
 * }
 * ```
 */
export function getPostProcessor(name: string): PostProcessor | undefined {
	return postProcessors.get(name);
}

/**
 * List all registered post-processor names
 *
 * @returns Array of processor names
 *
 * @example
 * ```typescript
 * const names = listPostProcessors();
 * console.log("Registered processors:", names);
 * ```
 */
export function listPostProcessors(): string[] {
	return Array.from(postProcessors.keys());
}

/**
 * Unregister a post-processor and call its shutdown method
 *
 * @param name - The processor name
 * @throws {Error} If the processor is not registered
 *
 * @example
 * ```typescript
 * await unregisterPostProcessor("my-processor");
 * ```
 */
export async function unregisterPostProcessor(name: string): Promise<void> {
	const processor = postProcessors.get(name);

	if (!processor) {
		const available = Array.from(postProcessors.keys());
		const availableStr = available.length > 0 ? ` Available: ${available.join(", ")}` : "";
		throw new Error(`Post-processor "${name}" is not registered.${availableStr}`);
	}

	try {
		if (processor.shutdown) {
			await processor.shutdown();
		}
	} catch (error) {
		console.warn(`Error during shutdown of post-processor "${name}":`, error);
	}

	postProcessors.delete(name);
}

/**
 * Clear all registered post-processors
 *
 * Calls shutdown on all processors before clearing.
 *
 * @example
 * ```typescript
 * await clearPostProcessors();
 * ```
 */
export async function clearPostProcessors(): Promise<void> {
	const entries = Array.from(postProcessors.entries());

	for (const [_name, processor] of entries) {
		try {
			if (processor.shutdown) {
				await processor.shutdown();
			}
		} catch (error) {
			console.warn(`Error during shutdown of post-processor "${_name}":`, error);
		}
	}

	postProcessors.clear();
}

// ============================================================================
// Validator Registry Functions
// ============================================================================

/**
 * Validate a validator object
 *
 * @throws {Error} If the validator doesn't implement required methods
 */
function validateValidator(validator: unknown): validator is Validator {
	if (validator === null || validator === undefined) {
		throw new Error("Validator cannot be null or undefined");
	}

	const obj = validator as Record<string, unknown>;

	if (typeof obj.name !== "function") {
		throw new Error("Validator must implement name() method");
	}

	if (typeof obj.validate !== "function") {
		throw new Error("Validator must implement validate() method");
	}

	const name = obj.name();
	if (typeof name !== "string" || name.trim() === "") {
		throw new Error("Validator name must be a non-empty string");
	}

	return true;
}

/**
 * Register a validator plugin
 *
 * @param validator - The validator to register
 * @throws {Error} If the validator is invalid or missing required methods
 *
 * @example
 * ```typescript
 * const validator = {
 *   name: () => "my-validator",
 *   priority: () => 50,
 *   validate: async (result) => {
 *     if (!result.content) {
 *       return { valid: false, errors: ["Content is empty"] };
 *     }
 *     return { valid: true, errors: [] };
 *   }
 * };
 * registerValidator(validator);
 * ```
 */
export function registerValidator(validator: Validator): void {
	validateValidator(validator);

	const name = validator.name();

	if (validators.has(name)) {
		console.warn(`Validator "${name}" already registered, overwriting with new implementation`);
	}

	validators.set(name, validator);
}

/**
 * Get a registered validator by name
 *
 * @param name - The validator name
 * @returns The validator, or undefined if not found
 *
 * @example
 * ```typescript
 * const validator = getValidator("my-validator");
 * if (validator) {
 *   console.log("Found validator:", validator.name());
 * }
 * ```
 */
export function getValidator(name: string): Validator | undefined {
	return validators.get(name);
}

/**
 * List all registered validator names
 *
 * @returns Array of validator names
 *
 * @example
 * ```typescript
 * const names = listValidators();
 * console.log("Registered validators:", names);
 * ```
 */
export function listValidators(): string[] {
	return Array.from(validators.keys());
}

/**
 * Unregister a validator and call its shutdown method
 *
 * @param name - The validator name
 * @throws {Error} If the validator is not registered
 *
 * @example
 * ```typescript
 * await unregisterValidator("my-validator");
 * ```
 */
export async function unregisterValidator(name: string): Promise<void> {
	const validator = validators.get(name);

	if (!validator) {
		const available = Array.from(validators.keys());
		const availableStr = available.length > 0 ? ` Available: ${available.join(", ")}` : "";
		throw new Error(`Validator "${name}" is not registered.${availableStr}`);
	}

	try {
		if (validator.shutdown) {
			await validator.shutdown();
		}
	} catch (error) {
		console.warn(`Error during shutdown of validator "${name}":`, error);
	}

	validators.delete(name);
}

/**
 * Clear all registered validators
 *
 * Calls shutdown on all validators before clearing.
 *
 * @example
 * ```typescript
 * await clearValidators();
 * ```
 */
export async function clearValidators(): Promise<void> {
	const entries = Array.from(validators.entries());

	for (const [_name, validator] of entries) {
		try {
			if (validator.shutdown) {
				await validator.shutdown();
			}
		} catch (error) {
			console.warn(`Error during shutdown of validator "${_name}":`, error);
		}
	}

	validators.clear();
}

// ============================================================================
// Global Callback Functions (for WASM module callbacks)
// ============================================================================

/**
 * Global callback for executing a post-processor from WASM
 *
 * Called by the WASM module to execute a registered post-processor.
 * Makes the callback available to WASM via the global scope.
 *
 * @internal
 */
export function executePostProcessor(name: string, result: ExtractionResult): Promise<ExtractionResult> {
	const processor = postProcessors.get(name);

	if (!processor) {
		return Promise.reject(new Error(`Post-processor "${name}" is not registered`));
	}

	try {
		const output = processor.process(result);

		if (output instanceof Promise) {
			return output;
		}

		return Promise.resolve(output);
	} catch (error) {
		return Promise.reject(new Error(`Error executing post-processor "${name}": ${String(error)}`));
	}
}

/**
 * Global callback for executing a validator from WASM
 *
 * Called by the WASM module to execute a registered validator.
 * Makes the callback available to WASM via the global scope.
 *
 * @internal
 */
export function executeValidator(
	name: string,
	result: ExtractionResult,
): Promise<{ valid: boolean; errors: string[] }> {
	const validator = validators.get(name);

	if (!validator) {
		return Promise.reject(new Error(`Validator "${name}" is not registered`));
	}

	try {
		const output = validator.validate(result);

		if (output instanceof Promise) {
			return output;
		}

		return Promise.resolve(output);
	} catch (error) {
		return Promise.reject(new Error(`Error executing validator "${name}": ${String(error)}`));
	}
}

/**
 * Expose global callback functions for WASM module
 *
 * This makes the plugin execution functions available as global callbacks
 * that the WASM module can invoke via JavaScript.
 *
 * @internal
 */
export function setupGlobalCallbacks(): void {
	// Make callbacks available to WASM module
	if (typeof globalThis !== "undefined") {
		const callbacksObj = globalThis as Record<string, unknown>;
		callbacksObj.__kreuzberg_execute_post_processor = executePostProcessor;
		callbacksObj.__kreuzberg_execute_validator = executeValidator;
	}
}

// Setup callbacks when module is imported
setupGlobalCallbacks();
