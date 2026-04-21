/**
 * Type conversion utilities for transforming native binding results to TypeScript types.
 *
 * This module handles conversion from raw native binding objects to strongly-typed
 * TypeScript structures, including metadata parsing and fallback value handling.
 *
 * @internal This module is part of the core infrastructure layer (Layer 1).
 */

import type {
	BoundingBox,
	Chunk,
	Element,
	ElementType,
	ExtractedImage,
	ExtractionResult,
	PageContent,
	Table,
} from "../types.js";

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
			chunkType: null,
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
	const metadata = (chunk["metadata"] as Record<string, unknown>) ?? {};
	return {
		content: (chunk["content"] as string) ?? "",
		chunkType: ((chunk["chunk_type"] ?? chunk["chunkType"]) as string | null) ?? null,
		embedding: (chunk["embedding"] as number[] | null) ?? null,
		metadata: {
			byteStart: ((metadata["byte_start"] ?? metadata["charStart"]) as number) ?? 0,
			byteEnd: ((metadata["byte_end"] ?? metadata["charEnd"]) as number) ?? 0,
			tokenCount: ((metadata["token_count"] ?? metadata["tokenCount"]) as number | null) ?? null,
			chunkIndex: ((metadata["chunk_index"] ?? metadata["chunkIndex"]) as number) ?? 0,
			totalChunks: ((metadata["total_chunks"] ?? metadata["totalChunks"]) as number) ?? 0,
			firstPage: ((metadata["first_page"] ?? metadata["firstPage"]) as number | null) ?? null,
			lastPage: ((metadata["last_page"] ?? metadata["lastPage"]) as number | null) ?? null,
			headingContext: (() => {
				const hc = (metadata["heading_context"] ?? metadata["headingContext"]) as
					| Record<string, unknown>
					| null
					| undefined;
				if (!hc) return null;
				const headings = hc["headings"];
				if (!Array.isArray(headings)) return null;
				return {
					headings: headings.map((h: unknown) => {
						const heading = h as Record<string, unknown>;
						return {
							level: (heading["level"] as number) ?? 0,
							text: (heading["text"] as string) ?? "",
						};
					}),
				};
			})(),
		},
	};
}

/**
 * Convert raw element object from native binding to typed Element.
 *
 * @param rawElement - Raw element object from native binding
 * @returns Typed Element object
 * @internal
 */
function convertElement(rawElement: unknown): Element {
	if (!rawElement || typeof rawElement !== "object") {
		return {
			elementId: "",
			elementType: "narrative_text",
			text: "",
			metadata: {},
		};
	}

	const element = rawElement as Record<string, unknown>;
	const elementMetadata = (element["metadata"] as Record<string, unknown>) ?? {};

	return {
		elementId: (element["element_id"] ?? element["elementId"] ?? "") as string,
		elementType: (element["element_type"] ?? element["elementType"] ?? "narrative_text") as ElementType,
		text: (element["text"] as string) ?? "",
		metadata: {
			pageNumber: ((elementMetadata["page_number"] ?? elementMetadata["pageNumber"]) as number | null) ?? null,
			filename: (elementMetadata["filename"] as string | null) ?? null,
			coordinates: elementMetadata["coordinates"] ? (elementMetadata["coordinates"] as BoundingBox) : null,
			elementIndex: ((elementMetadata["element_index"] ?? elementMetadata["elementIndex"]) as number | null) ?? null,
			additional: (elementMetadata["additional"] as Record<string, string>) ?? {},
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
		data: ensureUint8Array(image["data"]),
		format: (image["format"] as string) ?? "unknown",
		imageIndex: (image["imageIndex"] as number) ?? 0,
		pageNumber: (image["pageNumber"] as number | null) ?? null,
		width: (image["width"] as number | null) ?? null,
		height: (image["height"] as number | null) ?? null,
		colorspace: (image["colorspace"] as string | null) ?? null,
		bitsPerComponent: (image["bitsPerComponent"] as number | null) ?? null,
		isMask: (image["isMask"] as boolean) ?? false,
		description: (image["description"] as string | null) ?? null,
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
		pageNumber: (page["pageNumber"] as number) ?? 0,
		content: (page["content"] as string) ?? "",
		tables: Array.isArray(page["tables"]) ? (page["tables"] as Table[]) : [],
		images: Array.isArray(page["images"]) ? (page["images"] as unknown[]).map((image) => convertImage(image)) : [],
		isBlank: (page["isBlank"] as boolean | null | undefined) ?? null,
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
			elements: null,
			pages: null,
			document: null,
		};
	}

	const result = rawResult as Record<string, unknown>;
	const metadata = result["metadata"];
	const metadataValue =
		typeof metadata === "string" ? parseMetadata(metadata) : ((metadata as Record<string, unknown>) ?? {});

	const returnObj: ExtractionResult = {
		content: (result["content"] as string) ?? "",
		mimeType: (result["mimeType"] as string) ?? "application/octet-stream",
		metadata: metadataValue,
		tables: Array.isArray(result["tables"]) ? (result["tables"] as Table[]) : [],
		detectedLanguages: Array.isArray(result["detectedLanguages"]) ? (result["detectedLanguages"] as string[]) : null,
		chunks: null,
		images: null,
		elements: null,
		pages: null,
		document: (result["document"] as Record<string, unknown> | null) ?? null,
	};

	const chunksData = result["chunks"];
	if (Array.isArray(chunksData)) {
		returnObj.chunks = (chunksData as unknown[]).map((chunk) => convertChunk(chunk));
	}

	const imagesData = result["images"];
	if (Array.isArray(imagesData)) {
		returnObj.images = (imagesData as unknown[]).map((image) => convertImage(image));
	}

	const elementsData = result["elements"];
	if (Array.isArray(elementsData)) {
		returnObj.elements = (elementsData as unknown[]).map((element) => convertElement(element));
	}

	const pagesData = result["pages"];
	if (Array.isArray(pagesData)) {
		returnObj.pages = (pagesData as unknown[]).map((page) => convertPageContent(page));
	}

	const ocrElementsData = result["ocrElements"];
	if (Array.isArray(ocrElementsData)) {
		returnObj.ocrElements = ocrElementsData as import("../types.js").OcrElement[];
	}

	const extractedKeywordsData = result["extractedKeywords"];
	if (Array.isArray(extractedKeywordsData)) {
		returnObj.extractedKeywords = extractedKeywordsData as import("../types.js").ExtractedKeyword[];
	}

	const qualityScoreData = result["qualityScore"];
	if (typeof qualityScoreData === "number") {
		returnObj.qualityScore = qualityScoreData;
	}

	const processingWarningsData = result["processingWarnings"];
	if (Array.isArray(processingWarningsData)) {
		returnObj.processingWarnings = processingWarningsData as Array<{ source: string; message: string }>;
	}

	const annotationsData = result["annotations"];
	if (Array.isArray(annotationsData)) {
		returnObj.annotations = annotationsData as import("../types.js").PdfAnnotation[];
	}

	return returnObj;
}

/**
 * Export public conversion functions for use by extraction modules.
 */
export {
	convertChunk,
	convertElement,
	convertImage,
	convertPageContent,
	convertResult,
	ensureUint8Array,
	parseMetadata,
};
