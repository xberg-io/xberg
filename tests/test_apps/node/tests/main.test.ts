import { describe, it, expect, beforeAll, afterEach } from "vitest";
import { readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
	extractFile,
	extractFileSync,
	extractBytes,
	extractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
	batchExtractBytes,
	batchExtractBytesSync,
	ExtractionConfig,
	detectMimeType,
	detectMimeTypeFromPath,
	validateMimeType,
	getExtensionsForMime,
	registerPostProcessor,
	unregisterPostProcessor,
	clearPostProcessors,
	listPostProcessors,
	registerValidator,
	unregisterValidator,
	clearValidators,
	listValidators,
	registerOcrBackend,
	unregisterOcrBackend,
	clearOcrBackends,
	listOcrBackends,
	listDocumentExtractors,
	unregisterDocumentExtractor,
	clearDocumentExtractors,
	KreuzbergError,
	ValidationError,
	ParsingError,
	OcrError,
	CacheError,
	MissingDependencyError,
	ImageProcessingError,
	PluginError,
	ErrorCode,
	getLastErrorCode,
	getLastPanicContext,
	getErrorCodeName,
	getErrorCodeDescription,
	classifyError,
	listEmbeddingPresets,
	getEmbeddingPreset,
	__version__,
	type ExtractionResult,
	type Chunk,
	type ExtractedImage,
	type Table,
	type PostProcessorProtocol,
	type ValidatorProtocol,
	type OcrBackendProtocol,
	type OcrConfig,
	type ChunkingConfig,
	type PageConfig,
	type PdfConfig,
	type ImageExtractionConfig,
	type KeywordConfig,
	type LanguageDetectionConfig,
	type TokenReductionConfig,
	type PostProcessorConfig,
	type HtmlPreprocessingOptions,
	type HtmlConversionOptions,
	type PageContent,
	type ErrorClassification,
} from "@kreuzberg/node";

describe("Kreuzberg TypeScript/Node.js Bindings", () => {
	describe("Version Info", () => {
		it("should export version constant", () => {
			expect(__version__).toBeDefined();
			expect(typeof __version__).toBe("string");
			expect(__version__).toMatch(/^\d+\.\d+\.\d+/);
		});
	});

	describe("MIME Type Detection & Validation", () => {
		let testFile: string;

		beforeAll(() => {
			const buffer = Buffer.from("%PDF-1.4\n%EOF");
			testFile = join(tmpdir(), "test-pdf.pdf");
			writeFileSync(testFile, buffer);
		});

		it("should detect MIME type from bytes", () => {
			const pdfHeader = Buffer.from("%PDF-1.4");
			const mimeType = detectMimeType(pdfHeader);
			expect(typeof mimeType).toBe("string");
			expect(mimeType.length).toBeGreaterThan(0);
		});

		it("should detect MIME type from file path", () => {
			const mimeType = detectMimeTypeFromPath("document.pdf", false);
			expect(typeof mimeType).toBe("string");
			expect(mimeType.length).toBeGreaterThan(0);
		});

		it("should validate MIME type", () => {
			const result = validateMimeType("application/pdf");
			expect(typeof result).toBe("string");
			expect(result.length).toBeGreaterThan(0);
		});

		it("should get extensions for MIME type", () => {
			const extensions = getExtensionsForMime("application/pdf");
			expect(Array.isArray(extensions)).toBe(true);
			expect(extensions.length).toBeGreaterThan(0);
			expect(extensions[0]).toBe("pdf");
		});

		it("should handle multiple MIME type extensions", () => {
			const extensions = getExtensionsForMime(
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			);
			expect(Array.isArray(extensions)).toBe(true);
			expect(extensions.includes("docx")).toBe(true);
		});
	});

	describe("ExtractionConfig Builder", () => {
		it("should create default extraction config", () => {
			const config = ExtractionConfig.default();
			expect(config).toBeDefined();
			expect(typeof config).toBe("object");
		});

		it("should build config with custom options", () => {
			const config = ExtractionConfig.default().withChunking({ maxChars: 2048 }).build();
			expect(config).toBeDefined();
			expect(config.chunking).toBeDefined();
		});

		it("should configure OCR settings", () => {
			const ocrConfig: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};
			const config = ExtractionConfig.default().withOcr(ocrConfig).build();
			expect(config.ocr).toBeDefined();
			expect(config.ocr?.backend).toBe("tesseract");
		});

		it("should configure PDF settings", () => {
			const config = ExtractionConfig.default().withPdf({ extractTables: true }).build();
			expect(config.pdf).toBeDefined();
		});

		it("should configure chunking", () => {
			const chunkingConfig: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};
			const config = ExtractionConfig.default().withChunking(chunkingConfig).build();
			expect(config.chunking).toBeDefined();
			expect(config.chunking?.maxChars).toBe(4096);
		});

		it("should configure image extraction", () => {
			const imageConfig: ImageExtractionConfig = {
				enabled: true,
			};
			const config = ExtractionConfig.default().withImageExtraction(imageConfig).build();
			expect(config.imageExtraction).toBeDefined();
		});

		it("should configure keyword extraction", () => {
			const keywordConfig: KeywordConfig = {
				enabled: true,
				maxKeywords: 10,
			};
			const config = ExtractionConfig.default().withKeywords(keywordConfig).build();
			expect(config.keywords).toBeDefined();
		});

		it("should configure language detection", () => {
			const langConfig: LanguageDetectionConfig = {
				enabled: true,
			};
			const config = ExtractionConfig.default().withLanguageDetection(langConfig).build();
			expect(config.languageDetection).toBeDefined();
		});

		it("should enable/disable metadata extraction", () => {
			const config = ExtractionConfig.default().withMetadataExtraction(true).build();
			expect(config.metadataExtraction).toBe(true);
		});

		it("should enable/disable quality mode", () => {
			const config = ExtractionConfig.default().withQualityMode(true).build();
			expect(config.qualityMode).toBe(true);
		});
	});

	describe("Single Document Extraction", () => {
		let testPdfFile: string;
		let testTxtFile: string;
		let testPdfBuffer: Buffer;

		beforeAll(() => {
			testPdfBuffer = Buffer.from("%PDF-1.4\n%EOF");
			testPdfFile = join(tmpdir(), "test-document.pdf");
			writeFileSync(testPdfFile, testPdfBuffer);

			testTxtFile = join(tmpdir(), "test-document.txt");
			writeFileSync(testTxtFile, Buffer.from("Hello, World!"));
		});

		it("should extract from text file (sync)", () => {
			const config = ExtractionConfig.default().build();
			const result = extractFileSync(testTxtFile, config);
			expect(result).toBeDefined();
			expect(typeof result).toBe("object");
			validateExtractionResult(result);
		});

		it("should extract from text file (async)", async () => {
			const config = ExtractionConfig.default().build();
			const result = await extractFile(testTxtFile, config);
			expect(result).toBeDefined();
			expect(typeof result).toBe("object");
			validateExtractionResult(result);
		});

		it("should extract from bytes (sync)", () => {
			const config = ExtractionConfig.default().build();
			const result = extractBytesSync(testTxtFile, "text/plain", config);
			expect(result).toBeDefined();
			expect(typeof result).toBe("object");
			validateExtractionResult(result);
		});

		it("should extract from bytes (async)", async () => {
			const config = ExtractionConfig.default().build();
			const result = await extractBytes(testTxtFile, "text/plain", config);
			expect(result).toBeDefined();
			expect(typeof result).toBe("object");
			validateExtractionResult(result);
		});

		it("should extract with null config (uses defaults)", () => {
			const result = extractFileSync(testTxtFile, null);
			expect(result).toBeDefined();
			validateExtractionResult(result);
		});

		it("should handle missing file gracefully", () => {
			expect(() => {
				extractFileSync("/nonexistent/file.pdf", null);
			}).toThrow();
		});

		it("should handle invalid file path gracefully", async () => {
			await expect(extractFile("", null)).rejects.toThrow();
		});
	});

	describe("Batch Document Extraction", () => {
		const testFiles: string[] = [];
		const testBuffers: Buffer[] = [];
		const testMimeTypes: string[] = [];

		beforeAll(() => {
			for (let i = 0; i < 3; i++) {
				const filePath = join(tmpdir(), `batch-test-${i}.txt`);
				const content = Buffer.from(`Test document ${i}\n`);
				writeFileSync(filePath, content);
				testFiles.push(filePath);
				testBuffers.push(content);
				testMimeTypes.push("text/plain");
			}
		});

		it("should batch extract files (sync)", () => {
			const config = ExtractionConfig.default().build();
			const results = batchExtractFilesSync(testFiles, config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testFiles.length);
			for (const result of results) {
				validateExtractionResult(result);
			}
		});

		it("should batch extract files (async)", async () => {
			const config = ExtractionConfig.default().build();
			const results = await batchExtractFiles(testFiles, config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testFiles.length);
			for (const result of results) {
				validateExtractionResult(result);
			}
		});

		it("should batch extract bytes (sync)", () => {
			const config = ExtractionConfig.default().build();
			const results = batchExtractBytesSync(testBuffers, testMimeTypes, config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testBuffers.length);
			for (const result of results) {
				validateExtractionResult(result);
			}
		});

		it("should batch extract bytes (async)", async () => {
			const config = ExtractionConfig.default().build();
			const results = await batchExtractBytes(testBuffers, testMimeTypes, config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testBuffers.length);
			for (const result of results) {
				validateExtractionResult(result);
			}
		});

		it("should handle empty batch gracefully", () => {
			const config = ExtractionConfig.default().build();
			const results = batchExtractFilesSync([], config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(0);
		});

		it("should handle single item batch", () => {
			const config = ExtractionConfig.default().build();
			const results = batchExtractFilesSync([testFiles[0]], config);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(1);
		});
	});

	describe("Post-Processor Plugin Registry", () => {
		const mockProcessor: PostProcessorProtocol = {
			name: "test-processor",
			processContent: (content: string) => content.toUpperCase(),
		};

		afterEach(() => {
			clearPostProcessors();
		});

		it("should register a post-processor", () => {
			registerPostProcessor(mockProcessor);
			const list = listPostProcessors();
			expect(list).toContain("test-processor");
		});

		it("should list registered post-processors", () => {
			const list = listPostProcessors();
			expect(Array.isArray(list)).toBe(true);
		});

		it("should unregister a post-processor", () => {
			registerPostProcessor(mockProcessor);
			unregisterPostProcessor("test-processor");
			const list = listPostProcessors();
			expect(list).not.toContain("test-processor");
		});

		it("should clear all post-processors", () => {
			registerPostProcessor(mockProcessor);
			clearPostProcessors();
			const list = listPostProcessors();
			expect(list.length).toBe(0);
		});

		it("should handle duplicate registration", () => {
			registerPostProcessor(mockProcessor);
			registerPostProcessor(mockProcessor);
			const list = listPostProcessors();
			const count = list.filter((name) => name === "test-processor").length;
			expect(count).toBeGreaterThanOrEqual(1);
		});
	});

	describe("Validator Plugin Registry", () => {
		const mockValidator: ValidatorProtocol = {
			name: "test-validator",
			validate: (content: string) => ({ isValid: content.length > 0, errors: [] }),
		};

		afterEach(() => {
			clearValidators();
		});

		it("should register a validator", () => {
			registerValidator(mockValidator);
			const list = listValidators();
			expect(list).toContain("test-validator");
		});

		it("should list registered validators", () => {
			const list = listValidators();
			expect(Array.isArray(list)).toBe(true);
		});

		it("should unregister a validator", () => {
			registerValidator(mockValidator);
			unregisterValidator("test-validator");
			const list = listValidators();
			expect(list).not.toContain("test-validator");
		});

		it("should clear all validators", () => {
			registerValidator(mockValidator);
			clearValidators();
			const list = listValidators();
			expect(list.length).toBe(0);
		});

		it("should handle unregistering non-existent validator gracefully", () => {
			expect(() => unregisterValidator("non-existent-validator")).not.toThrow();
		});
	});

	describe("OCR Backend Plugin Registry", () => {
		const mockOcrBackend: OcrBackendProtocol = {
			name: "test-ocr",
			supportedLanguages: ["eng", "deu"],
			processImage: async (imageData: Uint8Array, config: OcrConfig) => ({
				text: "Test OCR output",
				confidence: 0.95,
			}),
		};

		afterEach(() => {
			clearOcrBackends();
		});

		it("should register an OCR backend", () => {
			registerOcrBackend(mockOcrBackend);
			const list = listOcrBackends();
			expect(list).toContain("test-ocr");
		});

		it("should list registered OCR backends", () => {
			const list = listOcrBackends();
			expect(Array.isArray(list)).toBe(true);
		});

		it("should unregister an OCR backend", () => {
			registerOcrBackend(mockOcrBackend);
			unregisterOcrBackend("test-ocr");
			const list = listOcrBackends();
			expect(list).not.toContain("test-ocr");
		});

		it("should clear all OCR backends", () => {
			registerOcrBackend(mockOcrBackend);
			clearOcrBackends();
			const list = listOcrBackends();
			expect(list.length).toBe(0);
		});
	});

	describe("Document Extractor Registry", () => {
		it("should list document extractors", () => {
			const extractors = listDocumentExtractors();
			expect(Array.isArray(extractors)).toBe(true);
			expect(extractors.length).toBeGreaterThanOrEqual(0);
		});

		it("should handle clearing document extractors", () => {
			clearDocumentExtractors();
			const extractors = listDocumentExtractors();
			expect(Array.isArray(extractors)).toBe(true);
		});
	});

	describe("Embedding Presets", () => {
		it("should list embedding presets", () => {
			const presets = listEmbeddingPresets();
			expect(Array.isArray(presets)).toBe(true);
		});

		it("should get specific embedding preset", () => {
			const presets = listEmbeddingPresets();
			if (presets.length > 0) {
				const preset = getEmbeddingPreset(presets[0]);
				if (preset) {
					expect(typeof preset).toBe("object");
				}
			}
		});

		it("should return null for non-existent preset", () => {
			const preset = getEmbeddingPreset("non-existent-preset-xyz");
			expect(preset).toBeNull();
		});
	});

	describe("Error Handling & Classification", () => {
		it("should have defined error codes", () => {
			expect(ErrorCode).toBeDefined();
			expect(typeof ErrorCode).toBe("object");
		});

		it("should get last error code", () => {
			const code = getLastErrorCode();
			expect(typeof code).toBe("number");
		});

		it("should get error code name", () => {
			const name = getErrorCodeName(0);
			expect(typeof name).toBe("string");
		});

		it("should get error code description", () => {
			const description = getErrorCodeDescription(0);
			expect(typeof description).toBe("string");
		});

		it("should classify error messages", () => {
			const classification = classifyError("PDF parsing error");
			expect(classification).toBeDefined();
			expect(typeof classification).toBe("object");
		});

		it("should get last panic context", () => {
			const context = getLastPanicContext();
			if (context !== null) {
				expect(typeof context).toBe("object");
			}
		});
	});

	describe("Error Exception Classes", () => {
		it("should have KreuzbergError", () => {
			expect(KreuzbergError).toBeDefined();
			const error = new KreuzbergError("test");
			expect(error).toBeInstanceOf(Error);
			expect(error.message).toBe("test");
		});

		it("should have ValidationError", () => {
			expect(ValidationError).toBeDefined();
			const error = new ValidationError("validation failed");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have ParsingError", () => {
			expect(ParsingError).toBeDefined();
			const error = new ParsingError("parsing failed");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have OcrError", () => {
			expect(OcrError).toBeDefined();
			const error = new OcrError("ocr failed");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have CacheError", () => {
			expect(CacheError).toBeDefined();
			const error = new CacheError("cache failed");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have MissingDependencyError", () => {
			expect(MissingDependencyError).toBeDefined();
			const error = new MissingDependencyError("dependency missing");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have ImageProcessingError", () => {
			expect(ImageProcessingError).toBeDefined();
			const error = new ImageProcessingError("image processing failed");
			expect(error).toBeInstanceOf(Error);
		});

		it("should have PluginError", () => {
			expect(PluginError).toBeDefined();
			const error = new PluginError("plugin failed");
			expect(error).toBeInstanceOf(Error);
		});
	});

	describe("Type System Validation", () => {
		it("should have ExtractionResult type", () => {
			const result: ExtractionResult = {
				content: "test",
				mimeType: "text/plain",
				chunks: [],
				images: [],
				tables: [],
				metadata: {},
				errors: [],
				duration: 0,
				pageCount: 1,
			};
			expect(result.content).toBe("test");
		});

		it("should have Chunk type", () => {
			const chunk: Chunk = {
				content: "test chunk",
				metadata: {
					byteStart: 0,
					byteEnd: 10,
					tokenCount: null,
					chunkIndex: 0,
					totalChunks: 1,
				},
				embedding: null,
			};
			expect(chunk.content).toBe("test chunk");
		});

		it("should have ExtractedImage type", () => {
			const image: ExtractedImage = {
				data: new Uint8Array(),
				format: "png",
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
			expect(image.format).toBe("png");
		});

		it("should have Table type", () => {
			const table: Table = {
				headers: ["col1", "col2"],
				rows: [["val1", "val2"]],
				metadata: {},
			};
			expect(table.headers.length).toBe(2);
		});

		it("should have PageContent type", () => {
			const pageContent: PageContent = {
				text: "page text",
				pageNumber: 1,
				bounds: null,
				metadata: {},
			};
			expect(pageContent.pageNumber).toBe(1);
		});

		it("should have OcrConfig type", () => {
			const ocrConfig: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};
			expect(ocrConfig.backend).toBe("tesseract");
		});

		it("should have ChunkingConfig type", () => {
			const chunkingConfig: ChunkingConfig = {
				maxChars: 2048,
				maxOverlap: 256,
			};
			expect(chunkingConfig.maxChars).toBe(2048);
		});

		it("should have PdfConfig type", () => {
			const pdfConfig: PdfConfig = {
				extractTables: true,
			};
			expect(pdfConfig.extractTables).toBe(true);
		});

		it("should have ImageExtractionConfig type", () => {
			const imageConfig: ImageExtractionConfig = {
				enabled: true,
			};
			expect(imageConfig.enabled).toBe(true);
		});

		it("should have KeywordConfig type", () => {
			const keywordConfig: KeywordConfig = {
				enabled: true,
			};
			expect(keywordConfig.enabled).toBe(true);
		});

		it("should have LanguageDetectionConfig type", () => {
			const langConfig: LanguageDetectionConfig = {
				enabled: true,
			};
			expect(langConfig.enabled).toBe(true);
		});

		it("should have ErrorClassification type", () => {
			const classification: ErrorClassification = {
				category: "parsing",
				severity: "high",
			};
			expect(classification.category).toBe("parsing");
		});
	});

	describe("Plugin Protocol Validation", () => {
		it("should satisfy PostProcessorProtocol", () => {
			const processor: PostProcessorProtocol = {
				name: "test",
				processContent: (content: string) => content,
			};
			expect(processor.name).toBe("test");
			expect(typeof processor.processContent).toBe("function");
		});

		it("should satisfy ValidatorProtocol", () => {
			const validator: ValidatorProtocol = {
				name: "test",
				validate: (content: string) => ({ isValid: true, errors: [] }),
			};
			expect(validator.name).toBe("test");
			expect(typeof validator.validate).toBe("function");
		});

		it("should satisfy OcrBackendProtocol", () => {
			const backend: OcrBackendProtocol = {
				name: "test",
				processImage: async (imageData: Uint8Array, config: OcrConfig) => ({
					text: "",
					confidence: 0,
				}),
			};
			expect(backend.name).toBe("test");
			expect(typeof backend.processImage).toBe("function");
		});
	});
});

function validateExtractionResult(result: unknown): void {
	expect(result).toBeDefined();
	expect(typeof result).toBe("object");

	if (typeof result === "object" && result !== null) {
		const r = result as Record<string, unknown>;

		if (r.content !== undefined) {
			expect(typeof r.content).toMatch(/^(string|object)$/);
		}

		if (r.chunks !== undefined) {
			expect(Array.isArray(r.chunks)).toBe(true);
		}

		if (r.images !== undefined) {
			expect(Array.isArray(r.images)).toBe(true);
		}

		if (r.tables !== undefined) {
			expect(Array.isArray(r.tables)).toBe(true);
		}

		if (r.metadata !== undefined) {
			expect(typeof r.metadata).toBe("object");
		}

		if (r.errors !== undefined) {
			expect(Array.isArray(r.errors)).toBe(true);
		}
	}
}
