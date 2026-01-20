import { getBinding } from "../core/binding.js";
import type { ExtractionResult, ValidatorProtocol } from "../types.js";

/**
 * Register a custom validator.
 *
 * Validators check extraction results for quality, completeness, or correctness.
 * Unlike post-processors, validator errors **fail fast** - if a validator throws an error,
 * the extraction fails immediately.
 *
 * Validators are async and run after post-processors in the extraction pipeline.
 *
 * @param validator - Validator implementing ValidatorProtocol
 *
 * @example
 * ```typescript
 * import { registerValidator, extractFile } from '@kreuzberg/node';
 *
 * class MinLengthValidator {
 *   name() {
 *     return 'min_length_validator';
 *   }
 *
 *   priority() {
 *     return 100;
 *   }
 *
 *   async validate(result) {
 *     if (result.content.length < 10) {
 *       throw new Error('Content too short');
 *     }
 *   }
 * }
 *
 * registerValidator(new MinLengthValidator());
 * ```
 */
export function registerValidator(validator: ValidatorProtocol): void {
	const binding = getBinding();

	const wrappedValidator = {
		name: typeof validator.name === "function" ? validator.name() : validator.name,
		priority: typeof validator.priority === "function" ? validator.priority() : validator.priority,
		async validate(...args: unknown[]): Promise<string> {
			const jsonString = args[0] as string;

			if (!jsonString || jsonString === "undefined") {
				throw new Error("Validator received invalid JSON string");
			}

			const wireResult = JSON.parse(jsonString);
			const result: ExtractionResult = {
				content: wireResult.content,
				mimeType: wireResult.mime_type,
				metadata: typeof wireResult.metadata === "string" ? JSON.parse(wireResult.metadata) : wireResult.metadata,
				tables: wireResult.tables || [],
				detectedLanguages: wireResult.detected_languages,
				chunks: wireResult.chunks,
				images: wireResult.images ?? null,
			};

			await Promise.resolve(validator.validate(result));
			return "";
		},
	};

	binding.registerValidator(wrappedValidator);
}

/**
 * Unregister a validator by name.
 *
 * Removes a previously registered validator from the global registry.
 * If the validator doesn't exist, this is a no-op (does not throw).
 *
 * @param name - Validator name to unregister (case-sensitive)
 *
 * @example
 * ```typescript
 * import { unregisterValidator } from '@kreuzberg/node';
 *
 * unregisterValidator('min_length_validator');
 * ```
 */
export function unregisterValidator(name: string): void {
	const binding = getBinding();
	binding.unregisterValidator(name);
}

/**
 * Clear all registered validators.
 *
 * Removes all validators from the global registry. Useful for test cleanup
 * or resetting state.
 *
 * @example
 * ```typescript
 * import { clearValidators } from '@kreuzberg/node';
 *
 * clearValidators();
 * ```
 */
export function clearValidators(): void {
	const binding = getBinding();
	binding.clearValidators();
}

/**
 * List all registered validators.
 *
 * Returns the names of all currently registered validators (both built-in and custom).
 *
 * @returns Array of validator names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listValidators } from '@kreuzberg/node';
 *
 * const names = listValidators();
 * console.log('Registered validators:', names);
 * ```
 */
export function listValidators(): string[] {
	const binding = getBinding();
	return binding.listValidators();
}

/**
 * Get a registered validator by name.
 *
 * Retrieves information about a specific validator from the registry.
 *
 * @param name - Name of the validator to retrieve
 * @returns The validator if found, null otherwise
 *
 * @example
 * ```typescript
 * import { getValidator } from '@kreuzberg/node';
 *
 * const validator = getValidator('min_length_validator');
 * if (validator) {
 *   console.log('Validator found:', validator.name);
 * }
 * ```
 */
export function getValidator(name: string): unknown {
	// Note: This function is not directly exposed by the native binding
	// It's a helper function that uses listValidators to check if a validator exists
	const validators = listValidators();
	return validators.includes(name) ? { name } : null;
}
