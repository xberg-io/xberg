/**
 * @internal Type assertion utilities.
 *
 * This module provides assertion functions for validating types at runtime.
 * These are pure utility functions with no dependencies on other internal modules.
 *
 * This is a Layer 0 module.
 */

/**
 * Asserts that a value is a Uint8Array and returns it.
 * Throws a TypeError if the value is not a Uint8Array.
 *
 * @param value The value to validate
 * @param name The name of the parameter (used in error messages)
 * @returns The validated Uint8Array
 * @throws TypeError if value is not a Uint8Array
 * @internal
 */
export function assertUint8Array(value: unknown, name: string): Uint8Array {
	if (!(value instanceof Uint8Array)) {
		throw new TypeError(`${name} must be a Uint8Array`);
	}
	return value;
}

/**
 * Asserts that a value is an array of Uint8Arrays and returns it.
 * Throws a TypeError if the value is not an array or contains non-Uint8Array elements.
 *
 * @param values The value to validate
 * @param name The name of the parameter (used in error messages)
 * @returns The validated array of Uint8Arrays
 * @throws TypeError if values is not an array or contains non-Uint8Array elements
 * @internal
 */
export function assertUint8ArrayList(values: unknown, name: string): Uint8Array[] {
	if (!Array.isArray(values)) {
		throw new TypeError(`${name} must be an array of Uint8Array`);
	}

	const array = values as unknown[];
	return array.map((value, index) => {
		try {
			return assertUint8Array(value, `${name}[${index}]`);
		} catch {
			throw new TypeError(`${name}[${index}] must be a Uint8Array`);
		}
	});
}

/**
 * Assertion that should never be reached - helps with type narrowing in exhaustive checks.
 * If this function is called, it indicates a logic error in the program.
 *
 * @param value The value that should have been handled by exhaustive checks
 * @throws Error Always throws with a message about the unreachable code
 * @internal
 */
export function assertUnreachable(value: never): never {
	throw new Error(`This code should be unreachable, but got: ${JSON.stringify(value)}`);
}

/**
 * Assertion that should never be reached - helps with type narrowing in exhaustive checks.
 * Alias for assertUnreachable - use this when you want a more semantic name.
 *
 * @param value The value that should have been handled by exhaustive checks
 * @throws Error Always throws with a message about the unreachable code
 * @internal
 */
export function assertNever(value: never): never {
	throw new Error(`Expected code to be unreachable but got: ${JSON.stringify(value)}`);
}
