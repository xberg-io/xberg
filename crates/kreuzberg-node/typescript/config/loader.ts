import { getBinding } from "../core/binding.js";
import type { ExtractionConfig as ExtractionConfigType } from "../types.js";

/**
 * ExtractionConfig namespace with static methods for loading configuration from files.
 *
 * Provides factory methods to load extraction configuration from TOML, YAML, or JSON files,
 * or to discover configuration files in the current directory tree.
 *
 * For creating configurations programmatically, use plain TypeScript objects instead:
 *
 * @example
 * ```typescript
 * import { ExtractionConfig, extractFile } from '@kreuzberg/node';
 *
 * // Load configuration from file
 * const config1 = ExtractionConfig.fromFile('config.toml');
 *
 * // Or create with plain object
 * const config2 = {
 *   chunking: { maxChars: 2048 },
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * };
 *
 * // Use with extraction
 * const result = await extractFile('document.pdf', null, config2);
 * ```
 */
export const ExtractionConfig = {
	/**
	 * Load extraction configuration from a file.
	 *
	 * Automatically detects the file format based on extension:
	 * - `.toml` - TOML format
	 * - `.yaml` - YAML format
	 * - `.json` - JSON format
	 *
	 * @param filePath - Path to the configuration file (absolute or relative)
	 * @returns ExtractionConfig object loaded from the file
	 *
	 * @throws {Error} If file does not exist or is not accessible
	 * @throws {Error} If file content is not valid TOML/YAML/JSON
	 * @throws {Error} If configuration structure is invalid
	 * @throws {Error} If file extension is not supported
	 *
	 * @example
	 * ```typescript
	 * import { ExtractionConfig } from '@kreuzberg/node';
	 *
	 * // Load from TOML file
	 * const config1 = ExtractionConfig.fromFile('kreuzberg.toml');
	 *
	 * // Load from YAML file
	 * const config2 = ExtractionConfig.fromFile('./config.yaml');
	 *
	 * // Load from JSON file
	 * const config3 = ExtractionConfig.fromFile('./config.json');
	 * ```
	 */
	fromFile(filePath: string): ExtractionConfigType {
		const binding = getBinding();
		return binding.loadExtractionConfigFromFile(filePath);
	},

	/**
	 * Discover and load configuration from current or parent directories.
	 *
	 * Searches for a `kreuzberg.toml` file starting from the current working directory
	 * and traversing up the directory tree. Returns the first configuration file found.
	 *
	 * @returns ExtractionConfig object if found, or null if no configuration file exists
	 *
	 * @example
	 * ```typescript
	 * import { ExtractionConfig } from '@kreuzberg/node';
	 *
	 * // Try to find config in current or parent directories
	 * const config = ExtractionConfig.discover();
	 * if (config) {
	 *   console.log('Found configuration');
	 *   // Use config for extraction
	 * } else {
	 *   console.log('No configuration file found, using defaults');
	 * }
	 * ```
	 */
	discover(): ExtractionConfigType | null {
		const binding = getBinding();
		return binding.discoverExtractionConfig();
	},
};

/**
 * Load extraction configuration from a file.
 *
 * @param filePath - Path to the configuration file
 * @returns ExtractionConfig object loaded from the file
 *
 * @deprecated Use ExtractionConfig.fromFile() instead
 */
export function loadConfigFile(filePath: string): ExtractionConfigType {
	return ExtractionConfig.fromFile(filePath);
}

/**
 * Load extraction configuration from a specified path.
 *
 * @param path - Path to the configuration file or directory
 * @returns ExtractionConfig object or null
 *
 * @deprecated Use ExtractionConfig.fromFile() or ExtractionConfig.discover() instead
 */
export function loadConfigFromPath(path: string): ExtractionConfigType | null {
	try {
		return ExtractionConfig.fromFile(path);
	} catch {
		return ExtractionConfig.discover();
	}
}
