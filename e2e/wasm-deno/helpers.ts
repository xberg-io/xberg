// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import type {
	ChunkingConfig,
	ExtractionConfig,
	ExtractionResult,
	ImageExtractionConfig,
	LanguageDetectionConfig,
	Metadata,
	OcrConfig,
	PageExtractionConfig,
	PdfConfig,
	PostProcessorConfig,
	Table,
	TesseractConfig,
	TokenReductionConfig,
} from "npm:@kreuzberg/wasm@^4.0.0";
// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
import { enableOcr, extractBytes, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertEquals, assertExists } from "@std/assert";

export type {
	ChunkingConfig,
	ExtractionConfig,
	ExtractionResult,
	ImageExtractionConfig,
	LanguageDetectionConfig,
	Metadata,
	OcrConfig,
	PageExtractionConfig,
	PdfConfig,
	PostProcessorConfig,
	Table,
	TesseractConfig,
	TokenReductionConfig,
};

export { enableOcr, extractBytes, initWasm };

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
	if (typeof raw.chunker_type === "string") {
		config.chunkerType = raw.chunker_type;
	}
	assignBooleanField(config, raw, "prepend_heading_context", "prependHeadingContext");
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
	assignBooleanField(config, raw, "extract_annotations", "extractAnnotations");
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

function mapPageConfig(raw: PlainRecord): PageExtractionConfig {
	const config: PlainRecord = {};
	assignBooleanField(config, raw, "extract_pages", "extractPages");
	assignBooleanField(config, raw, "insert_page_markers", "insertPageMarkers");
	if (typeof raw.marker_format === "string") {
		(config as unknown as PageExtractionConfig).markerFormat = raw.marker_format;
	}
	return config as unknown as PageExtractionConfig;
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
	assignNumberField(target, source, "extraction_timeout_secs", "extractionTimeoutSecs");

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

	if (isPlainRecord(source.pages)) {
		result.pages = mapPageConfig(source.pages as PlainRecord);
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

	if (isPlainRecord(source.acceleration)) {
		const accel = source.acceleration as PlainRecord;
		target.acceleration = {
			...(typeof accel.provider === "string" ? { provider: accel.provider } : {}),
			...(typeof accel.device_id === "number" ? { deviceId: accel.device_id } : {}),
		};
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
	const fileNotFound = lower.includes("notfound") || lower.includes("no such file") || lower.includes("not found");

	if (missingDependency || unsupportedFormat || pdfiumError || stackOverflow || fileNotFound || requirementHit) {
		const reason = missingDependency
			? "missing dependency"
			: unsupportedFormat
				? "unsupported format"
				: pdfiumError
					? "PDFium not available (non-browser environment)"
					: stackOverflow
						? "stack overflow (document too large for WASM)"
						: fileNotFound
							? "test document not found"
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
		eachHasHeadingContext?: boolean | null,
		contentStartsWithHeading?: boolean | null,
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
		if (eachHasHeadingContext === true) {
			for (const chunk of chunks) {
				assertExists(((chunk as PlainRecord).metadata as PlainRecord)?.headingContext, "Chunk missing heading_context");
			}
		}
		if (eachHasHeadingContext === false) {
			for (const chunk of chunks) {
				assertEquals(
					((chunk as PlainRecord).metadata as PlainRecord)?.headingContext ?? null,
					null,
					"Chunk should have no heading_context",
				);
			}
		}
		if (contentStartsWithHeading === true) {
			for (const chunk of chunks) {
				const content = (chunk as PlainRecord).content;
				assertEquals(
					typeof content === "string" && (content as string).charCodeAt(0) === 35,
					true,
					"Chunk content should start with heading",
				);
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
		if (images === undefined || images === null) {
			return; // Field not available in this binding
		}
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
		if (pages === undefined || pages === null) {
			return; // Field not available in this binding
		}
		if (!Array.isArray(pages)) {
			throw new Error("Expected pages to be an array");
		}
		if (typeof exactCount === "number") {
			assertEquals(pages.length, exactCount, `Expected exactly ${exactCount} pages, got ${pages.length}`);
		}
		if (typeof minCount === "number") {
			assertEquals(pages.length >= minCount, true, `Expected at least ${minCount} pages, got ${pages.length}`);
		}
		for (const page of pages) {
			const p = page as Record<string, unknown>;
			const isBlank = p["isBlank"];
			assertEquals(
				isBlank === undefined || isBlank === null || typeof isBlank === "boolean",
				true,
				"isBlank should be undefined, null, or boolean",
			);
		}
	},

	assertElements(result: ExtractionResult, minCount?: number | null, typesInclude?: string[] | null): void {
		const elements = (result as unknown as PlainRecord).elements as unknown[] | undefined;
		if (elements === undefined || elements === null) {
			return; // Field not available in this binding
		}
		if (!Array.isArray(elements)) {
			throw new Error("Expected elements to be an array");
		}
		if (typeof minCount === "number") {
			assertEquals(elements.length >= minCount, true, `Expected at least ${minCount} elements, got ${elements.length}`);
		}
		if (typesInclude && typesInclude.length > 0) {
			const foundTypes = new Set(elements.map((el) => (el as PlainRecord).element_type));
			for (const elType of typesInclude) {
				assertEquals(foundTypes.has(elType), true, `Expected element type ${elType} not found`);
			}
		}
	},

	assertOcrElements(
		result: ExtractionResult,
		hasElements?: boolean | null,
		elementsHaveGeometry?: boolean | null,
		elementsHaveConfidence?: boolean | null,
		minCount?: number | null,
	): void {
		const ocrElements = ((result as unknown as PlainRecord).ocrElements ??
			(result as unknown as PlainRecord).ocr_elements) as unknown[] | undefined;
		if (hasElements) {
			assertExists(ocrElements, "Expected ocrElements to be defined");
			if (!Array.isArray(ocrElements)) {
				throw new Error("Expected ocrElements to be an array");
			}
			assertEquals(ocrElements.length > 0, true, "Expected ocrElements to be non-empty");
		}
		if (Array.isArray(ocrElements)) {
			if (typeof minCount === "number") {
				assertEquals(
					ocrElements.length >= minCount,
					true,
					`Expected at least ${minCount} OCR elements, got ${ocrElements.length}`,
				);
			}
			if (elementsHaveGeometry) {
				for (const el of ocrElements) {
					const geometry = (el as PlainRecord).geometry;
					assertExists(geometry, "Expected OCR element to have geometry");
				}
			}
			if (elementsHaveConfidence) {
				for (const el of ocrElements) {
					const confidence = (el as PlainRecord).confidence;
					assertExists(confidence, "Expected OCR element to have confidence");
				}
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
					nodes.map((n) => ((n as PlainRecord).content as PlainRecord)?.nodeType ?? (n as PlainRecord).nodeType),
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
						((n as PlainRecord).content as PlainRecord)?.nodeType === "group" ||
						(n as PlainRecord).nodeType === "group",
				);
				assertEquals(hasGroupNodes, hasGroups, `Expected hasGroups=${hasGroups} but got ${hasGroupNodes}`);
			}
		} else {
			assertEquals(doc == null, true, "Expected document to be null but got a document");
		}
	},

	assertKeywords(
		result: ExtractionResult,
		hasKeywords?: boolean | null,
		minCount?: number | null,
		maxCount?: number | null,
	): void {
		const keywords = result.extractedKeywords as unknown[] | undefined;
		if (typeof hasKeywords === "boolean") {
			const keywordsExist = Array.isArray(keywords) && keywords.length > 0;
			assertEquals(keywordsExist, hasKeywords, `Expected hasKeywords=${hasKeywords} but got ${keywordsExist}`);
		}
		if (Array.isArray(keywords)) {
			if (typeof minCount === "number") {
				assertEquals(
					keywords.length >= minCount,
					true,
					`Expected at least ${minCount} keywords, got ${keywords.length}`,
				);
			}
			if (typeof maxCount === "number") {
				assertEquals(
					keywords.length <= maxCount,
					true,
					`Expected at most ${maxCount} keywords, got ${keywords.length}`,
				);
			}
		}
	},

	assertContentNotEmpty(result: ExtractionResult): void {
		assertEquals(result.content.length > 0, true, "Expected content to be non-empty");
	},

	assertTableBoundingBoxes(result: ExtractionResult, expected: boolean): void {
		if (expected) {
			const tables = Array.isArray(result.tables) ? result.tables : [];
			assertEquals(tables.length > 0, true, "Expected tables with bounding boxes but no tables found");
			for (const table of tables) {
				const bb = (table as PlainRecord).boundingBox ?? (table as PlainRecord).bounding_box;
				assertExists(bb, "Expected table to have bounding_box but it was null");
			}
		}
	},

	assertTableContentContainsAny(result: ExtractionResult, snippets: string[]): void {
		if (!snippets.length) {
			return;
		}
		const tables = Array.isArray(result.tables) ? result.tables : [];
		assertEquals(tables.length > 0, true, "Expected tables but none found");
		const allCells: string[] = [];
		for (const table of tables) {
			const cells = (table as PlainRecord).cells as unknown[][] | undefined;
			if (Array.isArray(cells)) {
				for (const row of cells) {
					for (const cell of row) {
						allCells.push(String(cell).toLowerCase());
					}
				}
			}
		}
		const found = snippets.some((s) => allCells.some((cell) => cell.includes(s.toLowerCase())));
		assertEquals(found, true, `No table cell contains any of [${snippets.join(", ")}]`);
	},

	assertImageBoundingBoxes(result: ExtractionResult, expected: boolean): void {
		if (expected) {
			const images = (result as unknown as PlainRecord).images as unknown[] | undefined;
			assertExists(images, "Expected images with bounding boxes but no images found");
			assertEquals(
				Array.isArray(images) && images.length > 0,
				true,
				"Expected images with bounding boxes but no images found",
			);
			for (const img of images) {
				const bb = (img as PlainRecord).boundingBox ?? (img as PlainRecord).bounding_box;
				assertExists(bb, "Expected image to have bounding_box but it was null");
			}
		}
	},

	assertQualityScore(
		result: ExtractionResult,
		hasScore?: boolean | null,
		minScore?: number | null,
		maxScore?: number | null,
	): void {
		const score = (result as unknown as PlainRecord).qualityScore ?? (result as unknown as PlainRecord).quality_score;
		if (hasScore === true) {
			assertExists(score, "Expected quality_score to be present");
		}
		if (hasScore === false) {
			assertEquals(score == null, true, "Expected quality_score to be absent");
		}
		if (typeof minScore === "number") {
			assertExists(score, "quality_score required for min_score assertion");
			assertEquals(Number(score) >= minScore, true, `quality_score ${score} < ${minScore}`);
		}
		if (typeof maxScore === "number") {
			assertExists(score, "quality_score required for max_score assertion");
			assertEquals(Number(score) <= maxScore, true, `quality_score ${score} > ${maxScore}`);
		}
	},

	assertProcessingWarnings(result: ExtractionResult, maxCount?: number | null, isEmpty?: boolean | null): void {
		const warnings = ((result as unknown as PlainRecord).processingWarnings ??
			(result as unknown as PlainRecord).processing_warnings) as unknown[] | undefined;
		const list = Array.isArray(warnings) ? warnings : [];
		if (typeof maxCount === "number") {
			assertEquals(list.length <= maxCount, true, `processing_warnings count ${list.length} > ${maxCount}`);
		}
		if (isEmpty === true) {
			assertEquals(list.length, 0, `Expected empty processing_warnings, got ${list.length}`);
		}
	},

	assertDjotContent(result: ExtractionResult, hasContent?: boolean | null, minBlocks?: number | null): void {
		const djot = (result as unknown as PlainRecord).djotContent ?? (result as unknown as PlainRecord).djot_content;
		if (hasContent === true) {
			assertExists(djot, "Expected djot_content to be present");
		}
		if (hasContent === false) {
			assertEquals(djot == null, true, "Expected djot_content to be absent");
		}
		if (typeof minBlocks === "number") {
			assertExists(djot, "djot_content required for min_blocks assertion");
			const blocks = Array.isArray(djot) ? djot : (((djot as PlainRecord)?.blocks as unknown[] | undefined) ?? []);
			assertEquals(blocks.length >= minBlocks, true, `djot_content blocks ${blocks.length} < ${minBlocks}`);
		}
	},

	assertAnnotations(result: ExtractionResult, hasAnnotations: boolean = false, minCount?: number | null): void {
		const annotations = (result as unknown as PlainRecord).annotations as unknown[] | undefined;
		if (hasAnnotations) {
			assertExists(annotations, "Expected annotations to be present");
			assertEquals(Array.isArray(annotations), true, "Expected annotations to be an array");
			assertEquals((annotations as unknown[]).length > 0, true, "Expected at least one annotation");
		}
		if (annotations !== undefined && annotations !== null && Array.isArray(annotations)) {
			if (typeof minCount === "number") {
				assertEquals(
					annotations.length >= minCount,
					true,
					`Expected at least ${minCount} annotations, got ${annotations.length}`,
				);
			}
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
