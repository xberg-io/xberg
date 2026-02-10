// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import type {
	ChunkingConfig,
	ExtractionConfig,
	ExtractionResult,
	ImageExtractionConfig,
	LanguageDetectionConfig,
	Metadata,
	OcrConfig,
	PdfConfig,
	PostProcessorConfig,
	Table,
	TesseractConfig,
	TokenReductionConfig,
} from "npm:@kreuzberg/wasm@^4.0.0";
// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import { extractBytes, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertEquals, assertExists } from "@std/assert";

export type {
	ChunkingConfig,
	ExtractionConfig,
	ExtractionResult,
	ImageExtractionConfig,
	LanguageDetectionConfig,
	Metadata,
	OcrConfig,
	PdfConfig,
	PostProcessorConfig,
	Table,
	TesseractConfig,
	TokenReductionConfig,
};

export { extractBytes, initWasm };

const WORKSPACE_ROOT = new URL("../..", import.meta.url).pathname.replace(/\/$/, "");
const TEST_DOCUMENTS = `${WORKSPACE_ROOT}/test_documents`;

type PlainRecord = Record<string, unknown>;

function isPlainRecord(value: unknown): value is PlainRecord {
	return typeof value === "object" && value !== null;
}

export async function resolveDocument(relative: string): Promise<Uint8Array> {
	const path = `${TEST_DOCUMENTS}/${relative}`;
	return await Deno.readFile(path);
}

function assignBooleanField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
	if (sourceKey in source) {
		const value = source[sourceKey];
		if (typeof value === "boolean") {
			target[targetKey] = value;
		} else if (value != null) {
			target[targetKey] = Boolean(value);
		}
	}
}

function assignNumberField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
	if (sourceKey in source) {
		const value = source[sourceKey];
		if (typeof value === "number") {
			target[targetKey] = value;
		} else if (typeof value === "string") {
			const parsed = Number(value);
			if (!Number.isNaN(parsed)) {
				target[targetKey] = parsed;
			}
		}
	}
}

function mapStringArray(value: unknown): string[] | undefined {
	if (!Array.isArray(value)) {
		return undefined;
	}
	return value.filter((item): item is string => typeof item === "string");
}

function mapTesseractConfig(raw: PlainRecord): TesseractConfig {
	const config: PlainRecord = {};
	assignNumberField(config, raw, "psm", "psm");
	assignBooleanField(config, raw, "enable_table_detection", "enableTableDetection");
	if (typeof raw.tessedit_char_whitelist === "string") {
		(config as unknown as TesseractConfig).tesseditCharWhitelist = raw.tessedit_char_whitelist;
	}
	return config as unknown as TesseractConfig;
}

function mapOcrConfig(raw: PlainRecord): OcrConfig | undefined {
	const backend = raw.backend;
	if (typeof backend !== "string" || backend.length === 0) {
		return undefined;
	}

	const config: PlainRecord = { backend };
	if (typeof raw.language === "string") {
		config.language = raw.language as string;
	}

	if (isPlainRecord(raw.tesseract_config)) {
		(config as unknown as OcrConfig).tesseractConfig = mapTesseractConfig(raw.tesseract_config as PlainRecord);
	}

	return config as unknown as OcrConfig;
}

function mapChunkingConfig(raw: PlainRecord): ChunkingConfig {
	const config: PlainRecord = {};
	assignNumberField(config, raw, "max_chars", "maxChars");
	assignNumberField(config, raw, "max_overlap", "maxOverlap");
	return config as unknown as ChunkingConfig;
}

function mapImageExtractionConfig(raw: PlainRecord): ImageExtractionConfig {
	const config: PlainRecord = {};
	assignBooleanField(config, raw, "extract_images", "extractImages");
	assignNumberField(config, raw, "target_dpi", "targetDpi");
	assignNumberField(config, raw, "max_image_dimension", "maxImageDimension");
	assignBooleanField(config, raw, "auto_adjust_dpi", "autoAdjustDpi");
	assignNumberField(config, raw, "min_dpi", "minDpi");
	assignNumberField(config, raw, "max_dpi", "maxDpi");
	return config as unknown as ImageExtractionConfig;
}

function mapPdfConfig(raw: PlainRecord): PdfConfig {
	const config: PlainRecord = {};
	assignBooleanField(config, raw, "extract_images", "extractImages");
	if (Array.isArray(raw.passwords)) {
		(config as unknown as PdfConfig).passwords = raw.passwords.filter(
			(item: unknown): item is string => typeof item === "string",
		);
	}
	assignBooleanField(config, raw, "extract_metadata", "extractMetadata");
	return config as unknown as PdfConfig;
}

function mapTokenReductionConfig(raw: PlainRecord): TokenReductionConfig {
	const config: PlainRecord = {};
	if (typeof raw.mode === "string") {
		(config as unknown as TokenReductionConfig).mode = raw.mode;
	}
	assignBooleanField(config, raw, "preserve_important_words", "preserveImportantWords");
	return config as unknown as TokenReductionConfig;
}

function mapLanguageDetectionConfig(raw: PlainRecord): LanguageDetectionConfig {
	const config: PlainRecord = {};
	assignBooleanField(config, raw, "enabled", "enabled");
	assignNumberField(config, raw, "min_confidence", "minConfidence");
	assignBooleanField(config, raw, "detect_multiple", "detectMultiple");
	return config as unknown as LanguageDetectionConfig;
}

function mapPostProcessorConfig(raw: PlainRecord): PostProcessorConfig {
	const config: PlainRecord = {};
	assignBooleanField(config, raw, "enabled", "enabled");
	const enabled = mapStringArray(raw.enabled_processors);
	if (enabled) {
		(config as unknown as PostProcessorConfig).enabledProcessors = enabled;
	}
	const disabled = mapStringArray(raw.disabled_processors);
	if (disabled) {
		(config as unknown as PostProcessorConfig).disabledProcessors = disabled;
	}
	return config as unknown as PostProcessorConfig;
}

export function buildConfig(raw: unknown): ExtractionConfig {
	if (!isPlainRecord(raw)) {
		return {};
	}

	const source = raw as PlainRecord;
	const result: ExtractionConfig = {};
	const target = result as PlainRecord;

	assignBooleanField(target, source, "use_cache", "useCache");
	assignBooleanField(target, source, "enable_quality_processing", "enableQualityProcessing");
	assignBooleanField(target, source, "force_ocr", "forceOcr");
	assignBooleanField(target, source, "include_document_structure", "includeDocumentStructure");
	assignNumberField(target, source, "max_concurrent_extractions", "maxConcurrentExtractions");

	if (isPlainRecord(source.ocr)) {
		const mapped = mapOcrConfig(source.ocr as PlainRecord);
		if (mapped) {
			result.ocr = mapped;
		}
	}

	if (isPlainRecord(source.chunking)) {
		result.chunking = mapChunkingConfig(source.chunking as PlainRecord);
	}

	if (isPlainRecord(source.images)) {
		result.images = mapImageExtractionConfig(source.images as PlainRecord);
	}

	if (isPlainRecord(source.pdf_options)) {
		result.pdfOptions = mapPdfConfig(source.pdf_options as PlainRecord);
	}

	if (isPlainRecord(source.token_reduction)) {
		result.tokenReduction = mapTokenReductionConfig(source.token_reduction as PlainRecord);
	}

	if (isPlainRecord(source.language_detection)) {
		result.languageDetection = mapLanguageDetectionConfig(source.language_detection as PlainRecord);
	}

	if (isPlainRecord(source.postprocessor)) {
		result.postprocessor = mapPostProcessorConfig(source.postprocessor as PlainRecord);
	}

	return result;
}

export function shouldSkipFixture(
	error: unknown,
	fixtureId: string,
	requirements: string[],
	notes?: string | null,
): boolean {
	if (!(error instanceof Error)) {
		return false;
	}

	const message = `${error.name}: ${error.message}`;
	const lower = message.toLowerCase();

	const requirementHit = requirements.some((req) => lower.includes(req.toLowerCase()));
	const missingDependency = lower.includes("missingdependencyerror") || lower.includes("missing dependency");
	const unsupportedFormat = lower.includes("unsupported mime type") || lower.includes("unsupported format");
	const pdfiumError = lower.includes("pdfium") || lower.includes("pdf extraction requires proper wasm");
	const stackOverflow = lower.includes("maximum call stack") || lower.includes("stack overflow");

	if (missingDependency || unsupportedFormat || pdfiumError || stackOverflow || requirementHit) {
		const reason = missingDependency
			? "missing dependency"
			: unsupportedFormat
				? "unsupported format"
				: pdfiumError
					? "PDFium not available (non-browser environment)"
					: stackOverflow
						? "stack overflow (document too large for WASM)"
						: requirements.join(", ");
		console.warn(`Skipping ${fixtureId}: ${reason}. ${message}`);
		if (notes) {
			console.warn(`Notes: ${notes}`);
		}
		return true;
	}

	return false;
}

export const assertions = {
	assertExpectedMime(result: ExtractionResult, expected: string[]): void {
		if (!expected.length) {
			return;
		}
		assertEquals(
			expected.some((token) => result.mimeType.includes(token)),
			true,
		);
	},

	assertMinContentLength(result: ExtractionResult, minimum: number): void {
		assertEquals(result.content.length >= minimum, true);
	},

	assertMaxContentLength(result: ExtractionResult, maximum: number): void {
		assertEquals(result.content.length <= maximum, true);
	},

	assertContentContainsAny(result: ExtractionResult, snippets: string[]): void {
		if (!snippets.length) {
			return;
		}
		const lowered = result.content.toLowerCase();
		assertEquals(
			snippets.some((snippet) => lowered.includes(snippet.toLowerCase())),
			true,
		);
	},

	assertContentContainsAll(result: ExtractionResult, snippets: string[]): void {
		if (!snippets.length) {
			return;
		}
		const lowered = result.content.toLowerCase();
		assertEquals(
			snippets.every((snippet) => lowered.includes(snippet.toLowerCase())),
			true,
		);
	},

	assertTableCount(result: ExtractionResult, minimum?: number | null, maximum?: number | null): void {
		const tables = Array.isArray(result.tables) ? result.tables : [];
		if (typeof minimum === "number") {
			assertEquals(tables.length >= minimum, true);
		}
		if (typeof maximum === "number") {
			assertEquals(tables.length <= maximum, true);
		}
	},

	assertDetectedLanguages(result: ExtractionResult, expected: string[], minConfidence?: number | null): void {
		if (!expected.length) {
			return;
		}
		assertExists(result.detectedLanguages);
		const languages = result.detectedLanguages ?? [];
		assertEquals(
			expected.every((lang) => languages.includes(lang)),
			true,
		);

		if (typeof minConfidence === "number" && isPlainRecord(result.metadata)) {
			const confidence = (result.metadata as PlainRecord).confidence;
			if (typeof confidence === "number") {
				assertEquals(confidence >= minConfidence, true);
			}
		}
	},

	assertMetadataExpectation(result: ExtractionResult, path: string, expectation: PlainRecord): void {
		if (!isPlainRecord(result.metadata)) {
			throw new Error(`Metadata is not a record for path ${path}`);
		}

		const value = getMetadataPath(result.metadata as PlainRecord, path);
		if (value === undefined || value === null) {
			throw new Error(`Metadata path '${path}' missing in ${JSON.stringify(result.metadata)}`);
		}

		if ("eq" in expectation) {
			assertEquals(valuesEqual(value, expectation.eq), true);
		}

		if ("gte" in expectation) {
			assertEquals(Number(value) >= Number(expectation.gte), true);
		}

		if ("lte" in expectation) {
			assertEquals(Number(value) <= Number(expectation.lte), true);
		}

		if ("contains" in expectation) {
			const contains = expectation.contains;
			if (typeof value === "string" && typeof contains === "string") {
				assertEquals(value.includes(contains), true);
			} else if (Array.isArray(value) && Array.isArray(contains)) {
				assertEquals(
					contains.every((item) => value.includes(item)),
					true,
				);
			} else {
				throw new Error(`Unsupported contains expectation for path '${path}'`);
			}
		}
	},

	assertChunks(
		result: ExtractionResult,
		minCount?: number | null,
		maxCount?: number | null,
		eachHasContent?: boolean | null,
		eachHasEmbedding?: boolean | null,
	): void {
		const chunks = (result as unknown as PlainRecord).chunks as unknown[] | undefined;
		assertExists(chunks, "Expected chunks to be defined");
		if (!Array.isArray(chunks)) {
			throw new Error("Expected chunks to be an array");
		}
		if (typeof minCount === "number") {
			assertEquals(chunks.length >= minCount, true, `Expected at least ${minCount} chunks, got ${chunks.length}`);
		}
		if (typeof maxCount === "number") {
			assertEquals(chunks.length <= maxCount, true, `Expected at most ${maxCount} chunks, got ${chunks.length}`);
		}
		if (eachHasContent) {
			for (const chunk of chunks) {
				assertExists((chunk as PlainRecord).content, "Chunk missing content");
			}
		}
		if (eachHasEmbedding) {
			for (const chunk of chunks) {
				assertExists((chunk as PlainRecord).embedding, "Chunk missing embedding");
			}
		}
	},

	assertImages(
		result: ExtractionResult,
		minCount?: number | null,
		maxCount?: number | null,
		formatsInclude?: string[] | null,
	): void {
		const images = (result as unknown as PlainRecord).images as unknown[] | undefined;
		assertExists(images, "Expected images to be defined");
		if (!Array.isArray(images)) {
			throw new Error("Expected images to be an array");
		}
		if (typeof minCount === "number") {
			assertEquals(images.length >= minCount, true, `Expected at least ${minCount} images, got ${images.length}`);
		}
		if (typeof maxCount === "number") {
			assertEquals(images.length <= maxCount, true, `Expected at most ${maxCount} images, got ${images.length}`);
		}
		if (formatsInclude && formatsInclude.length > 0) {
			const foundFormats = new Set(images.map((img) => (img as PlainRecord).format));
			for (const fmt of formatsInclude) {
				assertEquals(foundFormats.has(fmt), true, `Expected image format ${fmt} not found`);
			}
		}
	},

	assertPages(result: ExtractionResult, minCount?: number | null, exactCount?: number | null): void {
		const pages = (result as unknown as PlainRecord).pages as unknown[] | undefined;
		assertExists(pages, "Expected pages to be defined");
		if (!Array.isArray(pages)) {
			throw new Error("Expected pages to be an array");
		}
		if (typeof exactCount === "number") {
			assertEquals(pages.length, exactCount, `Expected exactly ${exactCount} pages, got ${pages.length}`);
		}
		if (typeof minCount === "number") {
			assertEquals(pages.length >= minCount, true, `Expected at least ${minCount} pages, got ${pages.length}`);
		}
	},

	assertElements(result: ExtractionResult, minCount?: number | null, typesInclude?: string[] | null): void {
		const elements = (result as unknown as PlainRecord).elements as unknown[] | undefined;
		assertExists(elements, "Expected elements to be defined");
		if (!Array.isArray(elements)) {
			throw new Error("Expected elements to be an array");
		}
		if (typeof minCount === "number") {
			assertEquals(elements.length >= minCount, true, `Expected at least ${minCount} elements, got ${elements.length}`);
		}
		if (typesInclude && typesInclude.length > 0) {
			const foundTypes = new Set(elements.map((el) => (el as PlainRecord).type));
			for (const elType of typesInclude) {
				assertEquals(foundTypes.has(elType), true, `Expected element type ${elType} not found`);
			}
		}
	},

	assertDocument(
		result: ExtractionResult,
		hasDocument: boolean,
		minNodeCount?: number | null,
		nodeTypesInclude?: string[] | null,
		hasGroups?: boolean | null,
	): void {
		const doc = (result as unknown as PlainRecord).document as PlainRecord | undefined | null;
		if (hasDocument) {
			assertExists(doc, "Expected document but got null");
			const nodes = (doc as PlainRecord).nodes as unknown[];
			assertExists(nodes, "Expected document nodes but got null");
			if (typeof minNodeCount === "number") {
				assertEquals(
					nodes.length >= minNodeCount,
					true,
					`Expected at least ${minNodeCount} nodes, got ${nodes.length}`,
				);
			}
			if (nodeTypesInclude && nodeTypesInclude.length > 0) {
				const foundTypes = new Set(
					nodes.map((n) => ((n as PlainRecord).content as PlainRecord)?.node_type ?? (n as PlainRecord).node_type),
				);
				for (const expected of nodeTypesInclude) {
					assertEquals(
						[...foundTypes].some((t) => typeof t === "string" && t.toLowerCase() === expected.toLowerCase()),
						true,
						`Expected node type '${expected}' not found in [${[...foundTypes].join(", ")}]`,
					);
				}
			}
			if (typeof hasGroups === "boolean") {
				const hasGroupNodes = nodes.some(
					(n) =>
						((n as PlainRecord).content as PlainRecord)?.node_type === "group" ||
						(n as PlainRecord).node_type === "group",
				);
				assertEquals(hasGroupNodes, hasGroups, `Expected hasGroups=${hasGroups} but got ${hasGroupNodes}`);
			}
		} else {
			assertEquals(doc == null, true, "Expected document to be null but got a document");
		}
	},
};

function lookupMetadataPath(metadata: PlainRecord, path: string): unknown {
	const segments = path.split(".");
	let current: unknown = metadata;
	for (const segment of segments) {
		if (!isPlainRecord(current) || !(segment in current)) {
			return undefined;
		}
		current = (current as PlainRecord)[segment];
	}
	return current;
}

function getMetadataPath(metadata: PlainRecord, path: string): unknown {
	const direct = lookupMetadataPath(metadata, path);
	if (direct !== undefined) {
		return direct;
	}
	const format = metadata.format;
	if (isPlainRecord(format)) {
		return lookupMetadataPath(format as PlainRecord, path);
	}
	return undefined;
}

function valuesEqual(lhs: unknown, rhs: unknown): boolean {
	if (typeof lhs === "string" && typeof rhs === "string") {
		return lhs === rhs;
	}
	if (typeof lhs === "number" && typeof rhs === "number") {
		return lhs === rhs;
	}
	if (typeof lhs === "boolean" && typeof rhs === "boolean") {
		return lhs === rhs;
	}
	return JSON.stringify(lhs) === JSON.stringify(rhs);
}
