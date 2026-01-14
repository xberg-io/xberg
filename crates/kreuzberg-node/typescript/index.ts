/**
 * Kreuzberg - Multi-language document intelligence framework.
 *
 * This is a TypeScript SDK around a high-performance Rust core.
 * All extraction logic, chunking, quality processing, and language detection
 * are implemented in Rust for maximum performance.
 *
 * ## API Usage Recommendations
 *
 * **For processing multiple documents**, prefer batch APIs:
 * - Use `batchExtractFiles()` / `batchExtractFilesSync()` for multiple files
 * - Use `batchExtractBytes()` / `batchExtractBytesSync()` for multiple byte arrays
 *
 * **Batch APIs provide**:
 * - Better performance (parallel processing in Rust)
 * - More reliable memory management
 * - Recommended for all multi-document workflows
 *
 * **Single extraction APIs** (`extractFile`, `extractBytes`) are suitable for:
 * - One-off document processing
 * - Interactive applications processing documents on-demand
 * - Avoid calling these in tight loops - use batch APIs instead
 *
 * ## Supported Formats
 *
 * - **Documents**: PDF, DOCX, PPTX, XLSX, DOC, PPT (with LibreOffice)
 * - **Text**: Markdown, Plain Text, XML
 * - **Web**: HTML (converted to Markdown)
 * - **Data**: JSON, YAML, TOML
 * - **Email**: EML, MSG
 * - **Images**: PNG, JPEG, TIFF (with OCR support)
 *
 * @example
 * ```typescript
 * import { extractFile, batchExtractFiles } from '@kreuzberg/node';
 *
 * // Single file extraction
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // Multiple files (recommended approach)
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = await batchExtractFiles(files);
 * results.forEach(r => console.log(r.content));
 * ```
 */

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import type { PanicContext } from "./errors.js";
import type {
	Chunk,
	ChunkingConfig,
	ErrorClassification,
	ExtractedImage,
	ExtractionConfig as ExtractionConfigType,
	ExtractionResult,
	HtmlConversionOptions,
	HtmlPreprocessingOptions,
	ImageExtractionConfig,
	KeywordConfig,
	LanguageDetectionConfig,
	OcrBackendProtocol,
	OcrConfig,
	PageContent,
	PageExtractionConfig,
	PdfConfig,
	PostProcessorConfig,
	PostProcessorProtocol,
	Table,
	TesseractConfig,
	TokenReductionConfig,
	ValidatorProtocol,
	WorkerPool,
	WorkerPoolStats,
} from "./types.js";

/**
 * @internal Native NAPI binding interface for the Kreuzberg native module.
 * This interface defines the shape of methods available in the compiled native addon.
 */
interface NativeBinding {
	extractFileSync(
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Record<string, unknown>;
	extractFile(
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	extractBytesSync(data: Buffer, mimeType: string, config: Record<string, unknown> | null): Record<string, unknown>;
	extractBytes(
		data: Buffer,
		mimeType: string,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	batchExtractFilesSync(paths: string[], config: Record<string, unknown> | null): Record<string, unknown>[];
	batchExtractFiles(paths: string[], config: Record<string, unknown> | null): Promise<Record<string, unknown>[]>;
	batchExtractBytesSync(
		dataArray: Buffer[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	): Record<string, unknown>[];
	batchExtractBytes(
		dataArray: Buffer[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>[]>;
	registerPostProcessor(processor: Record<string, unknown>): void;
	unregisterPostProcessor(name: string): void;
	clearPostProcessors(): void;
	listPostProcessors(): string[];
	registerValidator(validator: Record<string, unknown>): void;
	unregisterValidator(name: string): void;
	clearValidators(): void;
	listValidators(): string[];
	registerOcrBackend(backend: Record<string, unknown>): void;
	unregisterOcrBackend(name: string): void;
	clearOcrBackends(): void;
	listOcrBackends(): string[];
	registerDocumentExtractor(extractor: Record<string, unknown>): void;
	unregisterDocumentExtractor(name: string): void;
	clearDocumentExtractors(): void;
	listDocumentExtractors(): string[];
	detectMimeType(filePath: string): string;
	detectMimeTypeFromBytes(data: Buffer): string;
	detectMimeTypeFromPath(filePath: string, checkExists?: boolean): string;
	validateMimeType(mimeType: string): string;
	getExtensionsForMime(mimeType: string): string[];
	listEmbeddingPresets(): string[];
	getEmbeddingPreset(name: string): Record<string, unknown> | null;
	getErrorCodeName(code: number): string;
	getErrorCodeDescription(code: number): string;
	classifyError(errorMessage: string): Record<string, unknown>;
	getLastErrorCode(): number;
	getLastPanicContext(): Record<string, unknown> | null;
	loadExtractionConfigFromFile(filePath: string): Record<string, unknown>;
	discoverExtractionConfig(): Record<string, unknown> | null;
	createWorkerPool(size?: number): Record<string, unknown>;
	getWorkerPoolStats(pool: Record<string, unknown>): Record<string, unknown>;
	extractFileInWorker(
		pool: Record<string, unknown>,
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	batchExtractFilesInWorker(
		pool: Record<string, unknown>,
		paths: string[],
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>[]>;
	closeWorkerPool(pool: Record<string, unknown>): Promise<void>;
}

export {
	CacheError,
	ErrorCode,
	ImageProcessingError,
	KreuzbergError,
	MissingDependencyError,
	OcrError,
	type PanicContext,
	ParsingError,
	PluginError,
	ValidationError,
} from "./errors.js";
export { GutenOcrBackend } from "./ocr/guten-ocr.js";
export * from "./types.js";

let binding: NativeBinding | null = null;
let bindingInitialized = false;

function createNativeBindingError(error: unknown): Error {
	const hintParts: string[] = [];
	let detail = "Unknown error while requiring native module.";

	if (error instanceof Error) {
		detail = error.message || error.toString();
		if (/pdfium/i.test(detail)) {
			hintParts.push(
				"Pdfium runtime library was not found. Ensure the bundled libpdfium (dll/dylib/so) is present next to the native module.",
			);
		}
		return new Error(
			[
				"Failed to load Kreuzberg native bindings.",
				hintParts.length ? hintParts.join(" ") : "",
				"Report this error and attach the logs/stack trace for investigation.",
				`Underlying error: ${detail}`,
			]
				.filter(Boolean)
				.join(" "),
			{ cause: error },
		);
	}

	return new Error(
		[
			"Failed to load Kreuzberg native bindings.",
			"Report this error and attach the logs/stack trace for investigation.",
			`Underlying error: ${String(error)}`,
		].join(" "),
	);
}

function assertUint8Array(value: unknown, name: string): Uint8Array {
	if (!(value instanceof Uint8Array)) {
		throw new TypeError(`${name} must be a Uint8Array`);
	}
	return value;
}

function assertUint8ArrayList(values: unknown, name: string): Uint8Array[] {
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
 * @internal Allows tests to provide a mocked native binding.
 */
export function __setBindingForTests(mock: unknown): void {
	binding = mock as NativeBinding;
	bindingInitialized = true;
}

/**
 * @internal Resets the cached native binding for tests.
 */
export function __resetBindingForTests(): void {
	binding = null;
	bindingInitialized = false;
}

function loadNativeBinding(): NativeBinding {
	let localRequire: ((path: string) => unknown) | undefined;

	// In CJS, require is already available globally
	if (typeof require !== "undefined") {
		localRequire = require as (path: string) => unknown;
	} else {
		// In ESM, we need to create require from import.meta.url
		try {
			localRequire = createRequire(import.meta.url);
		} catch {
			localRequire = undefined;
		}
	}

	if (!localRequire) {
		throw new Error("Unable to resolve native binding loader (require not available).");
	}

	const loadedModule = localRequire("../index.js") as unknown;

	if (typeof loadedModule !== "object" || loadedModule === null) {
		throw new Error(
			"Native binding is not a valid object. " + "Ensure the native module is properly built and compatible.",
		);
	}

	const module = loadedModule as Record<string, unknown>;

	const requiredMethods = [
		"extractFileSync",
		"extractFile",
		"extractBytesSync",
		"extractBytes",
		"batchExtractFilesSync",
		"batchExtractFiles",
		"batchExtractBytesSync",
		"batchExtractBytes",
	];

	for (const method of requiredMethods) {
		if (typeof module[method] !== "function") {
			throw new Error(
				`Native binding is missing required method: ${method}. ` +
					"Ensure the native module is properly built and compatible.",
			);
		}
	}

	return module as unknown as NativeBinding;
}

function getBinding(): NativeBinding {
	if (bindingInitialized) {
		if (binding === null) {
			throw new Error("Native binding was previously failed to load.");
		}
		return binding;
	}

	try {
		if (typeof process !== "undefined" && process.versions && process.versions.node) {
			binding = loadNativeBinding();
			bindingInitialized = true;
			return binding;
		}
	} catch (error) {
		bindingInitialized = true;
		throw createNativeBindingError(error);
	}

	throw new Error(
		"Failed to load Kreuzberg bindings. Neither NAPI (Node.js) nor WASM (browsers/Deno) bindings are available. " +
			"Make sure you have installed the @kreuzberg/node package for Node.js/Bun.",
	);
}

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

type NativeExtractionConfig = Record<string, unknown>;

function setIfDefined<T>(target: NativeExtractionConfig, key: string, value: T | undefined): void {
	if (value !== undefined) {
		target[key] = value;
	}
}

function normalizeTesseractConfig(config?: TesseractConfig) {
	if (!config) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "psm", config.psm);
	setIfDefined(normalized, "enableTableDetection", config.enableTableDetection);
	setIfDefined(normalized, "tesseditCharWhitelist", config.tesseditCharWhitelist);
	return normalized;
}

function normalizeOcrConfig(ocr?: OcrConfig): NativeExtractionConfig | undefined {
	if (!ocr) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {
		backend: ocr.backend,
	};
	setIfDefined(normalized, "language", ocr.language);

	const tesseract = normalizeTesseractConfig(ocr.tesseractConfig);
	if (tesseract) {
		setIfDefined(normalized, "tesseractConfig", tesseract);
	}

	return normalized;
}

function normalizeChunkingConfig(chunking?: ChunkingConfig): NativeExtractionConfig | undefined {
	if (!chunking) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "maxChars", chunking.maxChars);
	setIfDefined(normalized, "maxOverlap", chunking.maxOverlap);
	setIfDefined(normalized, "preset", chunking.preset);
	setIfDefined(normalized, "embedding", chunking.embedding);
	setIfDefined(normalized, "enabled", chunking.enabled);
	return normalized;
}

function normalizeImageExtractionConfig(images?: ImageExtractionConfig): NativeExtractionConfig | undefined {
	if (!images) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractImages", images.extractImages);
	setIfDefined(normalized, "targetDpi", images.targetDpi);
	setIfDefined(normalized, "maxImageDimension", images.maxImageDimension);
	setIfDefined(normalized, "autoAdjustDpi", images.autoAdjustDpi);
	setIfDefined(normalized, "minDpi", images.minDpi);
	setIfDefined(normalized, "maxDpi", images.maxDpi);
	return normalized;
}

function normalizePdfConfig(pdf?: PdfConfig): NativeExtractionConfig | undefined {
	if (!pdf) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractImages", pdf.extractImages);
	setIfDefined(normalized, "passwords", pdf.passwords);
	setIfDefined(normalized, "extractMetadata", pdf.extractMetadata);
	return normalized;
}

function normalizeTokenReductionConfig(tokenReduction?: TokenReductionConfig): NativeExtractionConfig | undefined {
	if (!tokenReduction) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "mode", tokenReduction.mode);
	setIfDefined(normalized, "preserveImportantWords", tokenReduction.preserveImportantWords);
	return normalized;
}

function normalizeLanguageDetectionConfig(
	languageDetection?: LanguageDetectionConfig,
): NativeExtractionConfig | undefined {
	if (!languageDetection) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", languageDetection.enabled);
	setIfDefined(normalized, "minConfidence", languageDetection.minConfidence);
	setIfDefined(normalized, "detectMultiple", languageDetection.detectMultiple);
	return normalized;
}

function normalizePostProcessorConfig(postprocessor?: PostProcessorConfig): NativeExtractionConfig | undefined {
	if (!postprocessor) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", postprocessor.enabled);
	setIfDefined(normalized, "enabledProcessors", postprocessor.enabledProcessors);
	setIfDefined(normalized, "disabledProcessors", postprocessor.disabledProcessors);
	return normalized;
}

function normalizeHtmlPreprocessing(options?: HtmlPreprocessingOptions): NativeExtractionConfig | undefined {
	if (!options) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "enabled", options.enabled);
	setIfDefined(normalized, "preset", options.preset);
	setIfDefined(normalized, "removeNavigation", options.removeNavigation);
	setIfDefined(normalized, "removeForms", options.removeForms);
	return normalized;
}

function normalizeHtmlOptions(options?: HtmlConversionOptions): NativeExtractionConfig | undefined {
	if (!options) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "headingStyle", options.headingStyle);
	setIfDefined(normalized, "listIndentType", options.listIndentType);
	setIfDefined(normalized, "listIndentWidth", options.listIndentWidth);
	setIfDefined(normalized, "bullets", options.bullets);
	setIfDefined(normalized, "strongEmSymbol", options.strongEmSymbol);
	setIfDefined(normalized, "escapeAsterisks", options.escapeAsterisks);
	setIfDefined(normalized, "escapeUnderscores", options.escapeUnderscores);
	setIfDefined(normalized, "escapeMisc", options.escapeMisc);
	setIfDefined(normalized, "escapeAscii", options.escapeAscii);
	setIfDefined(normalized, "codeLanguage", options.codeLanguage);
	setIfDefined(normalized, "autolinks", options.autolinks);
	setIfDefined(normalized, "defaultTitle", options.defaultTitle);
	setIfDefined(normalized, "brInTables", options.brInTables);
	setIfDefined(normalized, "hocrSpatialTables", options.hocrSpatialTables);
	setIfDefined(normalized, "highlightStyle", options.highlightStyle);
	setIfDefined(normalized, "extractMetadata", options.extractMetadata);
	setIfDefined(normalized, "whitespaceMode", options.whitespaceMode);
	setIfDefined(normalized, "stripNewlines", options.stripNewlines);
	setIfDefined(normalized, "wrap", options.wrap);
	setIfDefined(normalized, "wrapWidth", options.wrapWidth);
	setIfDefined(normalized, "convertAsInline", options.convertAsInline);
	setIfDefined(normalized, "subSymbol", options.subSymbol);
	setIfDefined(normalized, "supSymbol", options.supSymbol);
	setIfDefined(normalized, "newlineStyle", options.newlineStyle);
	setIfDefined(normalized, "codeBlockStyle", options.codeBlockStyle);
	setIfDefined(normalized, "keepInlineImagesIn", options.keepInlineImagesIn);
	setIfDefined(normalized, "encoding", options.encoding);
	setIfDefined(normalized, "debug", options.debug);
	setIfDefined(normalized, "stripTags", options.stripTags);
	setIfDefined(normalized, "preserveTags", options.preserveTags);

	const preprocessing = normalizeHtmlPreprocessing(options.preprocessing);
	setIfDefined(normalized, "preprocessing", preprocessing);

	return normalized;
}

function normalizeKeywordConfig(config?: KeywordConfig): NativeExtractionConfig | undefined {
	if (!config) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "algorithm", config.algorithm);
	setIfDefined(normalized, "maxKeywords", config.maxKeywords);
	setIfDefined(normalized, "minScore", config.minScore);
	setIfDefined(normalized, "ngramRange", config.ngramRange);
	setIfDefined(normalized, "language", config.language);
	setIfDefined(normalized, "yakeParams", config.yakeParams);
	setIfDefined(normalized, "rakeParams", config.rakeParams);
	return normalized;
}

function normalizePageConfig(pages?: PageExtractionConfig): NativeExtractionConfig | undefined {
	if (!pages) {
		return undefined;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "extractPages", pages.extractPages);
	setIfDefined(normalized, "insertPageMarkers", pages.insertPageMarkers);
	setIfDefined(normalized, "markerFormat", pages.markerFormat);
	return normalized;
}

function normalizeExtractionConfig(config: ExtractionConfigType | null): NativeExtractionConfig | null {
	if (!config) {
		return null;
	}

	const normalized: NativeExtractionConfig = {};
	setIfDefined(normalized, "useCache", config.useCache);
	setIfDefined(normalized, "enableQualityProcessing", config.enableQualityProcessing);
	setIfDefined(normalized, "forceOcr", config.forceOcr);
	setIfDefined(normalized, "maxConcurrentExtractions", config.maxConcurrentExtractions);

	const ocr = normalizeOcrConfig(config.ocr);
	setIfDefined(normalized, "ocr", ocr);

	const chunking = normalizeChunkingConfig(config.chunking);
	setIfDefined(normalized, "chunking", chunking);

	const images = normalizeImageExtractionConfig(config.images);
	setIfDefined(normalized, "images", images);

	const pdf = normalizePdfConfig(config.pdfOptions);
	setIfDefined(normalized, "pdfOptions", pdf);

	const tokenReduction = normalizeTokenReductionConfig(config.tokenReduction);
	setIfDefined(normalized, "tokenReduction", tokenReduction);

	const languageDetection = normalizeLanguageDetectionConfig(config.languageDetection);
	setIfDefined(normalized, "languageDetection", languageDetection);

	const postprocessor = normalizePostProcessorConfig(config.postprocessor);
	setIfDefined(normalized, "postprocessor", postprocessor);

	const keywords = normalizeKeywordConfig(config.keywords);
	setIfDefined(normalized, "keywords", keywords);

	const pages = normalizePageConfig(config.pages);
	setIfDefined(normalized, "pages", pages);

	const htmlOptions = normalizeHtmlOptions(config.htmlOptions);
	setIfDefined(normalized, "htmlOptions", htmlOptions);

	return normalized;
}

/**
 * Extract content from a single file (synchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFilesSync()` which
 * provides better performance and memory management.
 *
 * @param filePath - Path to the file to extract (string). Can be absolute or relative.
 * @param mimeType - Optional MIME type hint for format detection. If null, MIME type is auto-detected from file extension or content.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns ExtractionResult containing extracted content, metadata, tables, and optional chunks/images
 * @throws {Error} If file doesn't exist, cannot be accessed, or cannot be read
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractFileSync } from '@kreuzberg/node';
 *
 * // Basic usage
 * const result = extractFileSync('document.pdf');
 * console.log(result.content);
 *
 * // With OCR configuration
 * const config = {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'eng',
 *     tesseractConfig: {
 *       psm: 6,
 *       enableTableDetection: true,
 *     },
 *   },
 * };
 * const result2 = extractFileSync('scanned.pdf', null, config);
 * ```
 */
export function extractFileSync(
	filePath: string,
	mimeTypeOrConfig?: string | null | ExtractionConfigType,
	maybeConfig?: ExtractionConfigType | null,
): ExtractionResult {
	let mimeType: string | null = null;
	let config: ExtractionConfigType | null = null;

	if (typeof mimeTypeOrConfig === "string") {
		mimeType = mimeTypeOrConfig;
		config = maybeConfig ?? null;
	} else if (mimeTypeOrConfig !== null && typeof mimeTypeOrConfig === "object") {
		config = mimeTypeOrConfig;
		mimeType = null;
	} else {
		config = maybeConfig ?? null;
		mimeType = null;
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = getBinding().extractFileSync(filePath, mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from a single file (asynchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFiles()` which
 * provides better performance and memory management.
 *
 * @param filePath - Path to the file to extract (string). Can be absolute or relative.
 * @param mimeType - Optional MIME type hint for format detection. If null, MIME type is auto-detected from file extension or content.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise<ExtractionResult> containing extracted content, metadata, tables, and optional chunks/images
 * @throws {Error} If file doesn't exist, cannot be accessed, or cannot be read
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractFile } from '@kreuzberg/node';
 *
 * // Basic usage
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // With chunking enabled
 * const config = {
 *   chunking: {
 *     maxChars: 1000,
 *     maxOverlap: 200,
 *   },
 * };
 * const result2 = await extractFile('long_document.pdf', null, config);
 * console.log(result2.chunks); // Array of text chunks
 * ```
 */
export async function extractFile(
	filePath: string,
	mimeTypeOrConfig?: string | null | ExtractionConfigType,
	maybeConfig?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	let mimeType: string | null = null;
	let config: ExtractionConfigType | null = null;

	if (typeof mimeTypeOrConfig === "string") {
		mimeType = mimeTypeOrConfig;
		config = maybeConfig ?? null;
	} else if (mimeTypeOrConfig !== null && typeof mimeTypeOrConfig === "object") {
		config = mimeTypeOrConfig;
		mimeType = null;
	} else {
		config = maybeConfig ?? null;
		mimeType = null;
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = await getBinding().extractFile(filePath, mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from raw bytes (synchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytesSync()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array (Buffer will be converted)
 * @param mimeType - MIME type of the data (required for accurate format detection). Must be a valid MIME type string.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns ExtractionResult containing extracted content, metadata, tables, and optional chunks/images
 * @throws {TypeError} When data is not a valid Uint8Array
 * @throws {Error} When file cannot be read or parsed
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractBytesSync } from '@kreuzberg/node';
 * import { readFileSync } from 'fs';
 *
 * const data = readFileSync('document.pdf');
 * const result = extractBytesSync(data, 'application/pdf');
 * console.log(result.content);
 * ```
 */
export function extractBytesSync(
	dataOrPath: Uint8Array | string,
	mimeType: string,
	config: ExtractionConfigType | null = null,
): ExtractionResult {
	let data: Uint8Array;
	if (typeof dataOrPath === "string") {
		data = readFileSync(dataOrPath);
	} else {
		data = dataOrPath;
	}

	const validated = assertUint8Array(data, "data");
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = getBinding().extractBytesSync(Buffer.from(validated), mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from raw bytes (asynchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytes()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array (Buffer will be converted)
 * @param mimeType - MIME type of the data (required for accurate format detection). Must be a valid MIME type string.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise<ExtractionResult> containing extracted content, metadata, tables, and optional chunks/images
 * @throws {TypeError} When data is not a valid Uint8Array
 * @throws {Error} When file cannot be read or parsed
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractBytes } from '@kreuzberg/node';
 * import { readFile } from 'fs/promises';
 *
 * const data = await readFile('document.pdf');
 * const result = await extractBytes(data, 'application/pdf');
 * console.log(result.content);
 * ```
 */
export async function extractBytes(
	dataOrPath: Uint8Array | string,
	mimeType: string,
	config: ExtractionConfigType | null = null,
): Promise<ExtractionResult> {
	let data: Uint8Array;
	if (typeof dataOrPath === "string") {
		data = readFileSync(dataOrPath);
	} else {
		data = dataOrPath;
	}

	const validated = assertUint8Array(data, "data");
	// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
	if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
		console.log("[TypeScript] Debug input header:", Array.from(validated.slice(0, 8)));
	}
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = await getBinding().extractBytes(Buffer.from(validated), mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from multiple files in parallel (synchronous).
 *
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFileSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract (absolute or relative paths)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Array of ExtractionResults (one per file, in same order as input)
 * @throws {Error} If any file cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractFilesSync } from '@kreuzberg/node';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = batchExtractFilesSync(files);
 *
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
 */
export function batchExtractFilesSync(paths: string[], config: ExtractionConfigType | null = null): ExtractionResult[] {
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = getBinding().batchExtractFilesSync(paths, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple files in parallel (asynchronous).
 *
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFile()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract (absolute or relative paths)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise resolving to array of ExtractionResults (one per file, in same order as input)
 * @throws {Error} If any file cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractFiles } from '@kreuzberg/node';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const results = await batchExtractFiles(files, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
 */
export async function batchExtractFiles(
	paths: string[],
	config: ExtractionConfigType | null = null,
): Promise<ExtractionResult[]> {
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = await getBinding().batchExtractFiles(paths, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (synchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytesSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays (must be same length as mimeTypes)
 * @param mimeTypes - List of MIME types (one per data item, required for accurate format detection)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Array of ExtractionResults (one per data item, in same order as input)
 * @throws {TypeError} When dataList contains non-Uint8Array items or length mismatch with mimeTypes
 * @throws {Error} If any data cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractBytesSync } from '@kreuzberg/node';
 * import { readFileSync } from 'fs';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const dataList = files.map(f => readFileSync(f));
 * const mimeTypes = ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'];
 *
 * const results = batchExtractBytesSync(dataList, mimeTypes);
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
 */
export function batchExtractBytesSync(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfigType | null = null,
): ExtractionResult[] {
	const buffers = assertUint8ArrayList(dataList, "dataList").map((data) => Buffer.from(data));

	if (buffers.length !== mimeTypes.length) {
		throw new TypeError("dataList and mimeTypes must have the same length");
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = getBinding().batchExtractBytesSync(buffers, mimeTypes, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (asynchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytes()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays (must be same length as mimeTypes)
 * @param mimeTypes - List of MIME types (one per data item, required for accurate format detection)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise resolving to array of ExtractionResults (one per data item, in same order as input)
 * @throws {TypeError} When dataList contains non-Uint8Array items or length mismatch with mimeTypes
 * @throws {Error} If any data cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractBytes } from '@kreuzberg/node';
 * import { readFile } from 'fs/promises';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const dataList = await Promise.all(files.map(f => readFile(f)));
 * const mimeTypes = files.map(() => 'application/pdf');
 *
 * const results = await batchExtractBytes(dataList, mimeTypes, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
 */
export async function batchExtractBytes(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfigType | null = null,
): Promise<ExtractionResult[]> {
	const buffers = assertUint8ArrayList(dataList, "dataList").map((data) => Buffer.from(data));

	if (buffers.length !== mimeTypes.length) {
		throw new TypeError("dataList and mimeTypes must have the same length");
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = await getBinding().batchExtractBytes(buffers, mimeTypes, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Register a custom postprocessor.
 *
 * **IMPORTANT**: Custom processors only work with **async extraction functions**:
 * - ✅ `extractFile()`, `extractBytes()`, `batchExtractFiles()`, `batchExtractBytes()`
 * - ❌ `extractFileSync()`, `extractBytesSync()`, etc. (will skip custom processors)
 *
 * This limitation exists because sync extraction blocks the Node.js event loop,
 * preventing JavaScript callbacks from executing. For v4.0, use async extraction
 * when you need custom processors.
 *
 * @param processor - PostProcessorProtocol implementation with name(), process(), and optional processingStage()
 * @throws {Error} If processor is missing required methods (name or process)
 * @throws {Error} If processor name is empty string
 * @throws {Error} If a processor with the same name is already registered
 *
 * @example
 * ```typescript
 * import { registerPostProcessor, extractFile, ExtractionResult } from '@kreuzberg/node';
 *
 * class MyProcessor implements PostProcessorProtocol {
 *   name(): string {
 *     return 'my_processor';
 *   }
 *
 *   process(result: ExtractionResult): ExtractionResult {
 *     result.metadata.customField = 'custom_value';
 *     return result;
 *   }
 *
 *   processingStage(): 'early' | 'middle' | 'late' {
 *     return 'middle';
 *   }
 * }
 *
 * registerPostProcessor(new MyProcessor());
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
 * Register a custom validator.
 *
 * Validators check extraction results for quality, completeness, or correctness.
 * Unlike post-processors, validator errors **fail fast** - if a validator throws an error,
 * the extraction fails immediately.
 *
 * @param validator - ValidatorProtocol implementation with name(), validate(), and optional priority()/shouldValidate()
 * @throws {Error} If validator is missing required methods (name or validate)
 * @throws {Error} If validator name is empty string
 * @throws {Error} If a validator with the same name is already registered
 *
 * @example
 * ```typescript
 * import { registerValidator } from '@kreuzberg/node';
 *
 * class MinLengthValidator implements ValidatorProtocol {
 *   name(): string {
 *     return 'min_length_validator';
 *   }
 *
 *   priority(): number {
 *     return 100; // Run early
 *   }
 *
 *   validate(result: ExtractionResult): void {
 *     if (result.content.length < 100) {
 *       throw new Error('Content too short: minimum 100 characters required');
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
 * Register a custom OCR backend.
 *
 * This function registers a JavaScript OCR backend that will be used by Kreuzberg's
 * extraction pipeline when OCR is enabled. The backend must implement the
 * {@link OcrBackendProtocol} interface.
 *
 * ## Usage
 *
 * 1. Create a class implementing {@link OcrBackendProtocol}
 * 2. Call `initialize()` on your backend instance (if needed)
 * 3. Register the backend with `registerOcrBackend()`
 * 4. Use the backend name in extraction config
 *
 * ## Thread Safety
 *
 * The registered backend must be thread-safe as it may be called concurrently
 * from multiple Rust async tasks. Ensure your implementation handles concurrent
 * calls properly.
 *
 * @param backend - OcrBackendProtocol implementation with name(), supportedLanguages(), and processImage()
 * @throws {Error} If backend is missing required methods (name, supportedLanguages, or processImage)
 * @throws {Error} If backend name is empty string or contains invalid characters
 * @throws {Error} If a backend with the same name is already registered
 * @throws {Error} If registration fails due to FFI issues
 *
 * @example
 * ```typescript
 * import { GutenOcrBackend } from '@kreuzberg/node/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@kreuzberg/node';
 *
 * // Create and initialize backend
 * const backend = new GutenOcrBackend();
 * await backend.initialize();
 *
 * // Register with Kreuzberg
 * registerOcrBackend(backend);
 *
 * // Use in extraction
 * const result = await extractFile('scanned.pdf', null, {
 *   ocr: { backend: 'guten-ocr', language: 'en' }
 * });
 * console.log(result.content);
 * ```
 *
 * @example
 * ```typescript
 * // Custom OCR backend implementation
 * class MyOcrBackend implements OcrBackendProtocol {
 *   name(): string {
 *     return 'my-ocr';
 *   }
 *
 *   supportedLanguages(): string[] {
 *     return ['en', 'de', 'fr'];
 *   }
 *
 *   async processImage(imageBytes: Uint8Array, language: string) {
 *     const text = await myCustomOcrEngine(imageBytes, language);
 *     return {
 *       content: text,
 *       mime_type: 'text/plain',
 *       metadata: { confidence: 0.95, language },
 *       tables: []
 *     };
 *   }
 * }
 *
 * registerOcrBackend(new MyOcrBackend());
 * ```
 */
type OcrProcessPayload = Buffer | string;
type OcrProcessTuple = [OcrProcessPayload, string];
type NestedOcrProcessTuple = [OcrProcessTuple];

function isOcrProcessTuple(value: unknown): value is OcrProcessTuple {
	return (
		Array.isArray(value) &&
		value.length === 2 &&
		typeof value[1] === "string" &&
		(typeof value[0] === "string" || Buffer.isBuffer(value[0]) || value[0] instanceof Uint8Array)
	);
}

function isNestedOcrProcessTuple(value: unknown): value is NestedOcrProcessTuple {
	return Array.isArray(value) && value.length === 1 && isOcrProcessTuple(value[0]);
}

function describePayload(value: OcrProcessPayload) {
	if (typeof value === "string") {
		return { ctor: "String", length: value.length };
	}

	return { ctor: value.constructor?.name ?? "Buffer", length: value.length };
}

export function registerOcrBackend(backend: OcrBackendProtocol): void {
	const binding = getBinding();

	const wrappedBackend = {
		name: typeof backend.name === "function" ? backend.name() : backend.name,
		supportedLanguages:
			typeof backend.supportedLanguages === "function"
				? backend.supportedLanguages()
				: (backend.supportedLanguages ?? ["en"]),
		async processImage(
			...processArgs: [OcrProcessPayload | OcrProcessTuple | NestedOcrProcessTuple, string?]
		): Promise<string> {
			const [imagePayload, maybeLanguage] = processArgs;
			// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
			if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
				console.log("[registerOcrBackend] JS arguments", { length: processArgs.length });
				console.log("[registerOcrBackend] Raw args", {
					imagePayloadType: Array.isArray(imagePayload) ? "tuple" : typeof imagePayload,
					maybeLanguageType: typeof maybeLanguage,
					metadata: Array.isArray(imagePayload) ? { tupleLength: imagePayload.length } : describePayload(imagePayload),
				});
			}

			let rawBytes: OcrProcessPayload;
			let language = maybeLanguage;

			if (isNestedOcrProcessTuple(imagePayload)) {
				[rawBytes, language] = imagePayload[0];
			} else if (isOcrProcessTuple(imagePayload)) {
				[rawBytes, language] = imagePayload;
			} else {
				rawBytes = imagePayload;
			}

			if (typeof language !== "string") {
				throw new Error("OCR backend did not receive a language parameter");
			}

			// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
			if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
				const length = typeof rawBytes === "string" ? rawBytes.length : rawBytes.length;
				console.log(
					"[registerOcrBackend] Received payload",
					Array.isArray(imagePayload) ? "tuple" : typeof rawBytes,
					"ctor",
					describePayload(rawBytes).ctor,
					"length",
					length,
				);
			}

			const buffer = typeof rawBytes === "string" ? Buffer.from(rawBytes, "base64") : Buffer.from(rawBytes);
			const result = await backend.processImage(new Uint8Array(buffer), language);

			return JSON.stringify(result);
		},
	};

	binding.registerOcrBackend(wrappedBackend);
}

/**
 * List all registered OCR backends.
 *
 * Returns an array of names of all currently registered OCR backends,
 * including built-in backends like "tesseract".
 *
 * @returns Array of OCR backend names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listOcrBackends } from '@kreuzberg/node';
 *
 * const backends = listOcrBackends();
 * console.log(backends); // ['tesseract', 'my-custom-backend', ...]
 * ```
 */
export function listOcrBackends(): string[] {
	const binding = getBinding();
	return binding.listOcrBackends();
}

/**
 * Unregister an OCR backend by name.
 *
 * Removes the specified OCR backend from the registry. If the backend doesn't exist,
 * this operation is a no-op (does not throw an error).
 *
 * @param name - Name of the OCR backend to unregister
 *
 * @example
 * ```typescript
 * import { unregisterOcrBackend } from '@kreuzberg/node';
 *
 * // Unregister a custom backend
 * unregisterOcrBackend('my-custom-ocr');
 * ```
 */
export function unregisterOcrBackend(name: string): void {
	const binding = getBinding();
	binding.unregisterOcrBackend(name);
}

/**
 * Clear all registered OCR backends.
 *
 * Removes all OCR backends from the registry, including built-in backends.
 * Use with caution as this will make OCR functionality unavailable until
 * backends are re-registered. If no backends are registered, this is a no-op.
 *
 * @example
 * ```typescript
 * import { clearOcrBackends } from '@kreuzberg/node';
 *
 * clearOcrBackends();
 * ```
 */
export function clearOcrBackends(): void {
	const binding = getBinding();
	binding.clearOcrBackends();
}

/**
 * List all registered document extractors.
 *
 * Returns an array of names of all currently registered document extractors,
 * including built-in extractors for PDF, Office documents, images, etc.
 *
 * @returns Array of document extractor names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listDocumentExtractors } from '@kreuzberg/node';
 *
 * const extractors = listDocumentExtractors();
 * console.log(extractors); // ['PDFExtractor', 'ImageExtractor', ...]
 * ```
 */
export function listDocumentExtractors(): string[] {
	const binding = getBinding();
	return binding.listDocumentExtractors();
}

/**
 * Unregister a document extractor by name.
 *
 * Removes the specified document extractor from the registry. If the extractor
 * doesn't exist, this operation is a no-op (does not throw an error).
 *
 * @param name - Name of the document extractor to unregister
 *
 * @example
 * ```typescript
 * import { unregisterDocumentExtractor } from '@kreuzberg/node';
 *
 * // Unregister a custom extractor
 * unregisterDocumentExtractor('MyCustomExtractor');
 * ```
 */
export function unregisterDocumentExtractor(name: string): void {
	const binding = getBinding();
	binding.unregisterDocumentExtractor(name);
}

/**
 * Clear all registered document extractors.
 *
 * Removes all document extractors from the registry, including built-in extractors.
 * Use with caution as this will make document extraction unavailable until
 * extractors are re-registered.
 *
 * @example
 * ```typescript
 * import { clearDocumentExtractors } from '@kreuzberg/node';
 *
 * clearDocumentExtractors();
 * ```
 */
export function clearDocumentExtractors(): void {
	const binding = getBinding();
	binding.clearDocumentExtractors();
}

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
 * Detect MIME type from raw bytes.
 *
 * Uses content inspection (magic bytes) to determine MIME type.
 * This is more accurate than extension-based detection but requires
 * reading the file content.
 *
 * @param bytes - Raw file content as Buffer
 * @returns The detected MIME type string
 *
 * @throws {Error} If MIME type cannot be determined from content
 *
 * @example
 * ```typescript
 * import { detectMimeType } from '@kreuzberg/node';
 * import * as fs from 'fs';
 *
 * // Read file content
 * const content = fs.readFileSync('document.pdf');
 *
 * // Detect MIME type from bytes
 * const mimeType = detectMimeType(content);
 * console.log(mimeType); // 'application/pdf'
 * ```
 */
export function detectMimeType(bytes: Buffer): string {
	const binding = getBinding();
	return binding.detectMimeTypeFromBytes(bytes);
}

/**
 * Detect MIME type from a file path.
 *
 * Determines the MIME type based on the file extension in the provided path.
 * By default, checks if the file exists; can be disabled with checkExists parameter.
 *
 * @param filePath - The file path to detect MIME type from (e.g., 'document.pdf')
 * @param checkExists - Whether to verify the file exists (default: true)
 * @returns The detected MIME type as a string (e.g., 'application/pdf')
 *
 * @throws {Error} If MIME type cannot be determined from the file extension,
 * or if checkExists is true and the file does not exist
 *
 * @example
 * ```typescript
 * import { detectMimeTypeFromPath } from '@kreuzberg/node';
 *
 * // Detect MIME type from existing file
 * const mimeType = detectMimeTypeFromPath('/path/to/document.pdf');
 * console.log(mimeType); // 'application/pdf'
 *
 * // Detect without checking file existence
 * const mimeType2 = detectMimeTypeFromPath('document.docx', false);
 * console.log(mimeType2); // 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
 * ```
 */
export function detectMimeTypeFromPath(filePath: string, checkExists?: boolean): string {
	const binding = getBinding();
	return binding.detectMimeTypeFromPath(filePath, checkExists);
}

/**
 * Validate that a MIME type is supported by Kreuzberg.
 *
 * Checks if a MIME type is in the list of supported formats. Note that any
 * `image/*` MIME type is automatically considered valid.
 *
 * @param mimeType - The MIME type to validate (string)
 * @returns The validated MIME type (may be normalized)
 *
 * @throws {Error} If the MIME type is not supported
 *
 * @example
 * ```typescript
 * import { validateMimeType } from '@kreuzberg/node';
 *
 * // Validate supported type
 * const validated = validateMimeType('application/pdf');
 * console.log(validated); // 'application/pdf'
 *
 * // Validate custom image type
 * const validated2 = validateMimeType('image/custom-format');
 * console.log(validated2); // 'image/custom-format' (any image/* is valid)
 *
 * // Validate unsupported type (throws error)
 * try {
 *   validateMimeType('video/mp4');
 * } catch (err) {
 *   console.error(err); // Error: Unsupported format: video/mp4
 * }
 * ```
 */
export function validateMimeType(mimeType: string): string {
	const binding = getBinding();
	return binding.validateMimeType(mimeType);
}

/**
 * Get file extensions for a given MIME type.
 *
 * Returns an array of file extensions commonly associated with the specified
 * MIME type. For example, 'application/pdf' returns ['pdf'].
 *
 * @param mimeType - The MIME type to look up (e.g., 'application/pdf', 'image/jpeg')
 * @returns Array of file extensions (without leading dots)
 *
 * @throws {Error} If the MIME type is not recognized or supported
 *
 * @example
 * ```typescript
 * import { getExtensionsForMime } from '@kreuzberg/node';
 *
 * // Get extensions for PDF
 * const pdfExts = getExtensionsForMime('application/pdf');
 * console.log(pdfExts); // ['pdf']
 *
 * // Get extensions for JPEG
 * const jpegExts = getExtensionsForMime('image/jpeg');
 * console.log(jpegExts); // ['jpg', 'jpeg']
 * ```
 */
export function getExtensionsForMime(mimeType: string): string[] {
	const binding = getBinding();
	return binding.getExtensionsForMime(mimeType);
}

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
 * List all available embedding preset names.
 *
 * Returns an array of preset names that can be used with `getEmbeddingPreset`.
 *
 * @returns Array of 4 preset names: ["fast", "balanced", "quality", "multilingual"]
 *
 * @example
 * ```typescript
 * import { listEmbeddingPresets } from '@kreuzberg/node';
 *
 * const presets = listEmbeddingPresets();
 * console.log(presets); // ['fast', 'balanced', 'quality', 'multilingual']
 * ```
 */
export function listEmbeddingPresets(): string[] {
	const binding = getBinding();
	return binding.listEmbeddingPresets();
}

/**
 * Get a specific embedding preset by name.
 *
 * Returns a preset configuration object, or null if the preset name is not found.
 *
 * @param name - The preset name (case-sensitive)
 * @returns An `EmbeddingPreset` object or `null` if not found
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
 * Get the error code for the last FFI error.
 *
 * Returns the FFI error code as an integer. This is useful for programmatic error handling
 * and distinguishing between different types of failures in native code.
 *
 * Error codes:
 * - 0: Success (no error)
 * - 1: GenericError
 * - 2: Panic
 * - 3: InvalidArgument
 * - 4: IoError
 * - 5: ParsingError
 * - 6: OcrError
 * - 7: MissingDependency
 *
 * @returns The integer error code
 *
 * @example
 * ```typescript
 * import { extractFile, getLastErrorCode, ErrorCode } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   const code = getLastErrorCode();
 *   if (code === ErrorCode.Panic) {
 *     console.error('Native code panic detected');
 *   }
 * }
 * ```
 */
export function getLastErrorCode(): number {
	const binding = getBinding();
	return binding.getLastErrorCode();
}

/**
 * Get panic context information if the last error was a panic.
 *
 * Returns detailed information about a panic in native code, or null if the last error was not a panic.
 * This provides debugging information when native code panics.
 *
 * @returns A `PanicContext` object with file, line, function, message, and timestamp_secs, or null if no panic context is available
 *
 * @example
 * ```typescript
 * import { extractFile, getLastPanicContext } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   const context = getLastPanicContext();
 *   if (context) {
 *     console.error(`Panic at ${context.file}:${context.line}`);
 *     console.error(`In function: ${context.function}`);
 *     console.error(`Message: ${context.message}`);
 *   }
 * }
 * ```
 */
export function getLastPanicContext(): PanicContext | null {
	const binding = getBinding();
	const result = binding.getLastPanicContext();
	return result as unknown as PanicContext | null;
}

/**
 * Returns the human-readable name for an error code.
 *
 * Maps numeric error codes to their string names, providing a consistent way
 * to get error code names across all platforms.
 *
 * @param code - The numeric error code (0-7)
 * @returns The error code name as a string (e.g., "validation", "ocr", "unknown")
 *
 * @example
 * ```typescript
 * import { getErrorCodeName } from '@kreuzberg/node';
 *
 * const name = getErrorCodeName(0);  // returns "validation"
 * const name = getErrorCodeName(2);  // returns "ocr"
 * const name = getErrorCodeName(99); // returns "unknown"
 * ```
 */
export function getErrorCodeName(code: number): string {
	const binding = getBinding();
	return binding.getErrorCodeName(code);
}

/**
 * Returns the description for an error code.
 *
 * Retrieves user-friendly descriptions of error types from the FFI layer.
 *
 * @param code - The numeric error code (0-7)
 * @returns A brief description of the error type
 *
 * @example
 * ```typescript
 * import { getErrorCodeDescription } from '@kreuzberg/node';
 *
 * const desc = getErrorCodeDescription(0);  // returns "Input validation error"
 * const desc = getErrorCodeDescription(4);  // returns "File system I/O error"
 * const desc = getErrorCodeDescription(99); // returns "Unknown error code"
 * ```
 */
export function getErrorCodeDescription(code: number): string {
	const binding = getBinding();
	return binding.getErrorCodeDescription(code);
}

/**
 * Classifies an error message string into an error code category.
 *
 * This function analyzes the error message content and returns the most likely
 * error code (0-7) based on keyword patterns. Used to programmatically classify
 * errors for handling purposes.
 *
 * The classification is based on keyword matching:
 * - **Validation (0)**: Keywords like "invalid", "validation", "schema", "required"
 * - **Parsing (1)**: Keywords like "parsing", "corrupted", "malformed"
 * - **Ocr (2)**: Keywords like "ocr", "tesseract", "language", "model"
 * - **MissingDependency (3)**: Keywords like "not found", "missing", "dependency"
 * - **Io (4)**: Keywords like "file", "disk", "read", "write", "permission"
 * - **Plugin (5)**: Keywords like "plugin", "register", "extension"
 * - **UnsupportedFormat (6)**: Keywords like "unsupported", "format", "mime"
 * - **Internal (7)**: Keywords like "internal", "bug", "panic"
 *
 * @param errorMessage - The error message string to classify
 * @returns An object with the classification details
 *
 * @example
 * ```typescript
 * import { classifyError } from '@kreuzberg/node';
 *
 * const result = classifyError("PDF file is corrupted");
 * // Returns: { code: 1, name: "parsing", confidence: 0.95 }
 *
 * const result = classifyError("Tesseract not found");
 * // Returns: { code: 3, name: "missing_dependency", confidence: 0.9 }
 * ```
 */
export function classifyError(errorMessage: string): ErrorClassification {
	const binding = getBinding();
	const result = binding.classifyError(errorMessage);
	return result as unknown as ErrorClassification;
}

/**
 * Create a worker pool for concurrent file extraction.
 *
 * The worker pool manages a set of background worker threads that can process
 * extraction requests concurrently, improving throughput when handling multiple files.
 *
 * @param size - Optional number of worker threads (defaults to CPU count). Must be > 0
 * @returns A WorkerPool instance to use with extraction functions
 *
 * @throws {Error} If size is invalid or pool creation fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * // Create pool with 4 workers
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const result = await extractFileInWorker(pool, 'document.pdf');
 *   console.log(result.content);
 * } finally {
 *   // Always close the pool when done
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export function createWorkerPool(size?: number): WorkerPool {
	const binding = getBinding();
	const rawPool = binding.createWorkerPool(size);
	return rawPool as unknown as WorkerPool;
}

/**
 * Get statistics about a worker pool.
 *
 * Returns information about the pool's current state, including the number of active workers,
 * queued tasks, and total processed tasks.
 *
 * @param pool - The worker pool instance
 * @returns WorkerPoolStats with pool information
 *
 * @example
 * ```typescript
 * import { createWorkerPool, getWorkerPoolStats } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 * const stats = getWorkerPoolStats(pool);
 *
 * console.log(`Pool size: ${stats.size}`);
 * console.log(`Active workers: ${stats.activeWorkers}`);
 * console.log(`Queued tasks: ${stats.queuedTasks}`);
 * ```
 */
export function getWorkerPoolStats(pool: WorkerPool): WorkerPoolStats {
	const binding = getBinding();
	const rawStats = binding.getWorkerPoolStats(pool as unknown as Record<string, unknown>);
	return rawStats as unknown as WorkerPoolStats;
}

/**
 * Extract content from a single file using a worker pool (asynchronous).
 *
 * Submits an extraction task to the worker pool. The task is executed by one of the
 * available workers in the background, allowing other tasks to be processed concurrently.
 *
 * @param pool - The worker pool instance
 * @param filePath - Path to the file to extract
 * @param mimeTypeOrConfig - Optional MIME type or extraction configuration
 * @param maybeConfig - Optional extraction configuration (if second param is MIME type)
 * @returns Promise<ExtractionResult> containing extracted content and metadata
 *
 * @throws {Error} If the file cannot be read or extraction fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 *   const results = await Promise.all(
 *     files.map(f => extractFileInWorker(pool, f))
 *   );
 *
 *   results.forEach((r, i) => {
 *     console.log(`${files[i]}: ${r.content.substring(0, 100)}...`);
 *   });
 * } finally {
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function extractFileInWorker(
	pool: WorkerPool,
	filePath: string,
	mimeTypeOrConfig?: string | null | ExtractionConfigType,
	maybeConfig?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	let mimeType: string | null = null;
	let config: ExtractionConfigType | null = null;

	if (typeof mimeTypeOrConfig === "string") {
		mimeType = mimeTypeOrConfig;
		config = maybeConfig ?? null;
	} else if (mimeTypeOrConfig !== null && typeof mimeTypeOrConfig === "object") {
		config = mimeTypeOrConfig;
		mimeType = null;
	} else {
		config = maybeConfig ?? null;
		mimeType = null;
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const binding = getBinding();
	const rawResult = await binding.extractFileInWorker(
		pool as unknown as Record<string, unknown>,
		filePath,
		mimeType,
		normalizedConfig,
	);
	return convertResult(rawResult);
}

/**
 * Extract content from multiple files in parallel using a worker pool (asynchronous).
 *
 * Submits multiple extraction tasks to the worker pool for concurrent processing.
 * This is more efficient than using `extractFileInWorker` multiple times sequentially.
 *
 * @param pool - The worker pool instance
 * @param paths - Array of file paths to extract
 * @param config - Extraction configuration object (applies to all files)
 * @returns Promise<ExtractionResult[]> array of results (one per file, in same order)
 *
 * @throws {Error} If any file cannot be read or extraction fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, batchExtractFilesInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 *   const results = await batchExtractFilesInWorker(pool, files, {
 *     ocr: { backend: 'tesseract', language: 'eng' }
 *   });
 *
 *   const total = results.reduce((sum, r) => sum + extractAmount(r.content), 0);
 *   console.log(`Total: $${total}`);
 * } finally {
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function batchExtractFilesInWorker(
	pool: WorkerPool,
	paths: string[],
	config: ExtractionConfigType | null = null,
): Promise<ExtractionResult[]> {
	const normalizedConfig = normalizeExtractionConfig(config);
	const binding = getBinding();
	const rawResults = await binding.batchExtractFilesInWorker(
		pool as unknown as Record<string, unknown>,
		paths,
		normalizedConfig,
	);
	return rawResults.map(convertResult);
}

/**
 * Close a worker pool and shut down all worker threads.
 *
 * Should be called when the pool is no longer needed to clean up resources
 * and gracefully shut down worker threads. Any pending tasks will be cancelled.
 *
 * @param pool - The worker pool instance to close
 * @returns Promise that resolves when the pool is fully closed
 *
 * @throws {Error} If pool shutdown fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const result = await extractFileInWorker(pool, 'document.pdf');
 *   console.log(result.content);
 * } finally {
 *   // Clean up the pool
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function closeWorkerPool(pool: WorkerPool): Promise<void> {
	const binding = getBinding();
	await binding.closeWorkerPool(pool as unknown as Record<string, unknown>);
}

export const __version__ = "4.0.6";
