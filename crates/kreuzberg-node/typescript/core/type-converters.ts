/**
 * Type conversion utilities for transforming native binding results to TypeScript types.
 *
 * This module handles conversion from raw native binding objects to strongly-typed
 * TypeScript structures, including metadata parsing and fallback value handling.
 *
 * @internal This module is part of the core infrastructure layer (Layer 1).
 */

import type { Chunk, ExtractedImage, ExtractionResult, PageContent, Table } from "../types.js";

/**
 * Parse metadata JSON string to a record object.
 * Returns an empty object if parsing fails or input is not a string.
 *
 * @param metadataStr - JSON string to parse
 * @returns Parsed metadata object or empty record
 * @internal
 */
function parseMetadata(metadataStr: string): Record<string, unknown> {
	try {
		const parsed = JSON.parse(metadataStr) as unknown;
		if (typeof parsed === "object" && parsed !== null) {
			return parsed as Record<string, unknown>;
		}
		return {};
	} catch {
		return {};
	}
}

/**
 * Ensure a value is a Uint8Array, converting from Buffer or Array if needed.
 * Returns an empty Uint8Array if conversion fails.
 *
 * @param value - Value to convert
 * @returns Uint8Array instance
 * @internal
 */
function ensureUint8Array(value: unknown): Uint8Array {
	if (value instanceof Uint8Array) {
		return value;
	}
	if (typeof Buffer !== "undefined" && value instanceof Buffer) {
		return new Uint8Array(value);
	}
	if (Array.isArray(value)) {
		return new Uint8Array(value);
	}
	return new Uint8Array();
}

/**
 * Convert raw chunk object from native binding to typed Chunk.
 *
 * @param rawChunk - Raw chunk object from native binding
 * @returns Typed Chunk object
 * @internal
 */
function convertChunk(rawChunk: unknown): Chunk {
	if (!rawChunk || typeof rawChunk !== "object") {
		return {
			content: "",
			metadata: {
				byteStart: 0,
				byteEnd: 0,
				tokenCount: null,
				chunkIndex: 0,
				totalChunks: 0,
			},
			embedding: null,
		};
	}

	const chunk = rawChunk as Record<string, unknown>;
	// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
	const metadata = (chunk["metadata"] as Record<string, unknown>) ?? {};
	return {
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		content: (chunk["content"] as string) ?? "",
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		embedding: (chunk["embedding"] as number[] | null) ?? null,
		metadata: {
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			byteStart: ((metadata["byte_start"] ?? metadata["charStart"]) as number) ?? 0,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			byteEnd: ((metadata["byte_end"] ?? metadata["charEnd"]) as number) ?? 0,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			tokenCount: ((metadata["token_count"] ?? metadata["tokenCount"]) as number | null) ?? null,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			chunkIndex: ((metadata["chunk_index"] ?? metadata["chunkIndex"]) as number) ?? 0,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			totalChunks: ((metadata["total_chunks"] ?? metadata["totalChunks"]) as number) ?? 0,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			firstPage: ((metadata["first_page"] ?? metadata["firstPage"]) as number | null) ?? null,
			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
			lastPage: ((metadata["last_page"] ?? metadata["lastPage"]) as number | null) ?? null,
		},
	};
}

/**
 * Convert raw image object from native binding to typed ExtractedImage.
 *
 * @param rawImage - Raw image object from native binding
 * @returns Typed ExtractedImage object
 * @internal
 */
function convertImage(rawImage: unknown): ExtractedImage {
	if (!rawImage || typeof rawImage !== "object") {
		return {
			data: new Uint8Array(),
			format: "unknown",
			imageIndex: 0,
			pageNumber: null,
			width: null,
			height: null,
			colorspace: null,
			bitsPerComponent: null,
			isMask: false,
			description: null,
			ocrResult: null,
		};
	}

	const image = rawImage as Record<string, unknown>;
	return {
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		data: ensureUint8Array(image["data"]),
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		format: (image["format"] as string) ?? "unknown",
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		imageIndex: (image["imageIndex"] as number) ?? 0,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		pageNumber: (image["pageNumber"] as number | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		width: (image["width"] as number | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		height: (image["height"] as number | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		colorspace: (image["colorspace"] as string | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		bitsPerComponent: (image["bitsPerComponent"] as number | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		isMask: (image["isMask"] as boolean) ?? false,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		description: (image["description"] as string | null) ?? null,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		ocrResult: image["ocrResult"] ? convertResult(image["ocrResult"]) : null,
	};
}

/**
 * Convert raw page object from native binding to typed PageContent.
 *
 * @param rawPage - Raw page object from native binding
 * @returns Typed PageContent object
 * @internal
 */
function convertPageContent(rawPage: unknown): PageContent {
	if (!rawPage || typeof rawPage !== "object") {
		return {
			pageNumber: 0,
			content: "",
			tables: [],
			images: [],
		};
	}

	const page = rawPage as Record<string, unknown>;
	return {
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		pageNumber: (page["pageNumber"] as number) ?? 0,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		content: (page["content"] as string) ?? "",
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		tables: Array.isArray(page["tables"]) ? (page["tables"] as Table[]) : [],
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		images: Array.isArray(page["images"]) ? (page["images"] as unknown[]).map((image) => convertImage(image)) : [],
	};
}

/**
 * Convert raw result object from native binding to typed ExtractionResult.
 * Handles metadata parsing, array conversions, and nested structure conversion.
 *
 * @param rawResult - Raw result object from native binding
 * @returns Typed ExtractionResult object
 * @internal
 */
function convertResult(rawResult: unknown): ExtractionResult {
	if (!rawResult || typeof rawResult !== "object") {
		return {
			content: "",
			mimeType: "application/octet-stream",
			metadata: {},
			tables: [],
			detectedLanguages: null,
			chunks: null,
			images: null,
			pages: null,
		};
	}

	const result = rawResult as Record<string, unknown>;
	// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
	const metadata = result["metadata"];
	const metadataValue =
		typeof metadata === "string" ? parseMetadata(metadata) : ((metadata as Record<string, unknown>) ?? {});

	const returnObj: ExtractionResult = {
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		content: (result["content"] as string) ?? "",
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		mimeType: (result["mimeType"] as string) ?? "application/octet-stream",
		metadata: metadataValue,
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		tables: Array.isArray(result["tables"]) ? (result["tables"] as Table[]) : [],
		// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
		detectedLanguages: Array.isArray(result["detectedLanguages"]) ? (result["detectedLanguages"] as string[]) : null,
		chunks: null,
		images: null,
		pages: null,
	};

	// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
	const chunksData = result["chunks"];
	if (Array.isArray(chunksData)) {
		returnObj.chunks = (chunksData as unknown[]).map((chunk) => convertChunk(chunk));
	}

	// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
	const imagesData = result["images"];
	if (Array.isArray(imagesData)) {
		returnObj.images = (imagesData as unknown[]).map((image) => convertImage(image));
	}

	// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noPropertyAccessFromIndexSignature
	const pagesData = result["pages"];
	if (Array.isArray(pagesData)) {
		returnObj.pages = (pagesData as unknown[]).map((page) => convertPageContent(page));
	}

	return returnObj;
}

/**
 * Export public conversion functions for use by extraction modules.
 */
export { parseMetadata, ensureUint8Array, convertChunk, convertImage, convertPageContent, convertResult };
