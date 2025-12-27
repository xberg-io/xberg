/**
 * Result type definitions for Kreuzberg document extraction.
 *
 * These types represent the output of extraction operations,
 * including extracted content, metadata, tables, chunks, and images.
 */

import type { Metadata } from "./metadata.js";

// ============================================================================

export interface Table {
	cells: string[][];
	markdown: string;
	pageNumber: number;
}

export interface ChunkMetadata {
	charStart: number;
	charEnd: number;
	tokenCount?: number | null;
	chunkIndex: number;
	totalChunks: number;
}

export interface Chunk {
	content: string;
	embedding?: number[] | null;
	metadata: ChunkMetadata;
}

export interface ExtractedImage {
	data: Uint8Array;
	format: string;
	imageIndex: number;
	pageNumber?: number | null;
	width?: number | null;
	height?: number | null;
	colorspace?: string | null;
	bitsPerComponent?: number | null;
	isMask: boolean;
	description?: string | null;
	ocrResult?: ExtractionResult | null;
}

export interface ExtractionResult {
	content: string;
	mimeType: string;
	metadata: Metadata;
	tables: Table[];
	detectedLanguages: string[] | null;
	chunks: Chunk[] | null;
	images: ExtractedImage[] | null;

	/**
	 * Get the page count from this extraction result.
	 *
	 * Returns the total number of pages/slides/sheets detected
	 * in the original document.
	 *
	 * @returns The page count (>= 0), or null if not available
	 *
	 * @example
	 * ```typescript
	 * const result = await extractFile('document.pdf');
	 * const pageCount = result.getPageCount();
	 * if (pageCount !== null) {
	 *   console.log(`Document has ${pageCount} pages`);
	 * }
	 * ```
	 */
	getPageCount(): number | null;

	/**
	 * Get the chunk count from this extraction result.
	 *
	 * Returns the number of text chunks when chunking is enabled,
	 * or null if chunking was not performed or information is unavailable.
	 *
	 * @returns The chunk count (>= 0), or null if not available
	 *
	 * @example
	 * ```typescript
	 * const result = await extractFile('document.pdf', {
	 *   chunking: { enabled: true, maxChars: 1024 }
	 * });
	 * const chunkCount = result.getChunkCount();
	 * if (chunkCount !== null) {
	 *   console.log(`Document has ${chunkCount} chunks`);
	 * }
	 * ```
	 */
	getChunkCount(): number | null;

	/**
	 * Get the detected language from this extraction result.
	 *
	 * Returns the primary detected language as an ISO 639 language code
	 * (e.g., "en", "de", "fr"). If multiple languages were detected,
	 * returns the primary one.
	 *
	 * @returns The language code (e.g., "en"), or null if not detected
	 *
	 * @example
	 * ```typescript
	 * const result = await extractFile('document.pdf');
	 * const language = result.getDetectedLanguage();
	 * if (language) {
	 *   console.log(`Detected language: ${language}`);
	 * }
	 * ```
	 */
	getDetectedLanguage(): string | null;

	/**
	 * Get a metadata field from this extraction result.
	 *
	 * Retrieves a metadata field value. Supports nested fields with dot notation
	 * (e.g., "format.pages", "author").
	 *
	 * @param fieldName - The metadata field name or path to retrieve
	 * @returns The field value (parsed from JSON), or null if not found
	 *
	 * @example
	 * ```typescript
	 * const result = await extractFile('document.pdf');
	 *
	 * // Get simple field
	 * const title = result.getMetadataField('title') as string | null;
	 * if (title) {
	 *   console.log(`Title: ${title}`);
	 * }
	 *
	 * // Get nested field
	 * const pageCount = result.getMetadataField('format.pages') as number | null;
	 * if (pageCount !== null) {
	 *   console.log(`Pages: ${pageCount}`);
	 * }
	 * ```
	 */
	getMetadataField(fieldName: string): unknown;
}
