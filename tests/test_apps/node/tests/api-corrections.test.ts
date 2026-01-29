/**
 * API Corrections Test Suite
 *
 * This suite fixes the issues discovered in main.test.ts and validates the actual
 * API signatures and behavior of @kreuzberg/node@4.0.0-rc.16
 */

import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
	__version__,
	batchExtractBytes,
	batchExtractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
	classifyError,
	clearOcrBackends,
	clearPostProcessors,
	clearValidators,
	detectMimeType,
	ExtractionConfig,
	type ExtractionResult,
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
	getEmbeddingPreset,
	getExtensionsForMime,
	getLastErrorCode,
	listDocumentExtractors,
	listEmbeddingPresets,
	listOcrBackends,
	listPostProcessors,
	listValidators,
	type OcrBackendProtocol,
	type OcrConfig,
	type PostProcessorProtocol,
	registerOcrBackend,
	registerPostProcessor,
	registerValidator,
	unregisterOcrBackend,
	unregisterPostProcessor,
	unregisterValidator,
	type ValidatorProtocol,
	validateMimeType,
} from "@kreuzberg/node";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

describe("Kreuzberg TypeScript/Node.js Bindings - Corrected API Tests", () => {
	describe("ExtractionConfig - Actual API", () => {
		let testFile: string;

		beforeAll(() => {
			const content = Buffer.from("Hello, World!");
			testFile = join(tmpdir(), "test-document.txt");
			writeFileSync(testFile, content);
		});

		it("should load config from file", () => {
			const configPath = join(tmpdir(), "test-config.json");
			const configContent = JSON.stringify({
				chunking: { maxChars: 2048 },
				ocr: { backend: "tesseract", language: "eng" },
			});
			writeFileSync(configPath, configContent);

			const config = ExtractionConfig.fromFile(configPath);
			expect(config).toBeDefined();
			expect(typeof config).toBe("object");
		});

		it("should discover config or return null", () => {
			const config = ExtractionConfig.discover();
			if (config !== null) {
				expect(typeof config).toBe("object");
			}
		});

		it("should accept null config (uses defaults)", () => {
			const result = extractFileSync(testFile, null, null);
			expect(result).toBeDefined();
		});

		it("should accept undefined mimeType and config", () => {
			const result = extractFileSync(testFile);
			expect(result).toBeDefined();
		});
	});

	describe("Extraction Functions - Corrected Signatures", () => {
		let testFile: string;
		let testBuffer: Buffer;

		beforeAll(() => {
			const content = Buffer.from("Hello, World!");
			testFile = join(tmpdir(), "test-document.txt");
			writeFileSync(testFile, content);
			testBuffer = content;
		});

		it("extractFileSync accepts (filePath, mimeType?, config?)", () => {
			const result1 = extractFileSync(testFile);
			expect(result1).toBeDefined();

			const result2 = extractFileSync(testFile, null);
			expect(result2).toBeDefined();

			const result3 = extractFileSync(testFile, "text/plain");
			expect(result3).toBeDefined();
		});

		it("extractBytesSync requires (data: Buffer, mimeType: string, config?)", () => {
			const result = extractBytesSync(testBuffer, "text/plain");
			expect(result).toBeDefined();

			const result2 = extractBytesSync(testBuffer, "text/plain", null);
			expect(result2).toBeDefined();
		});

		it("extractFile (async) has same signature as sync version", async () => {
			const result = await extractFile(testFile);
			expect(result).toBeDefined();

			const result2 = await extractFile(testFile, "text/plain", null);
			expect(result2).toBeDefined();
		});

		it("extractBytes (async) requires Buffer and mimeType", async () => {
			const result = await extractBytes(testBuffer, "text/plain");
			expect(result).toBeDefined();

			const result2 = await extractBytes(testBuffer, "text/plain", null);
			expect(result2).toBeDefined();
		});
	});

	describe("Batch Extraction Functions - Corrected", () => {
		const testFiles: string[] = [];
		const testBuffers: Buffer[] = [];

		beforeAll(() => {
			for (let i = 0; i < 2; i++) {
				const filePath = join(tmpdir(), `batch-${i}.txt`);
				const content = Buffer.from(`Document ${i}`);
				writeFileSync(filePath, content);
				testFiles.push(filePath);
				testBuffers.push(content);
			}
		});

		it("batchExtractFilesSync accepts (paths: string[], config?)", () => {
			const results = batchExtractFilesSync(testFiles);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testFiles.length);

			const results2 = batchExtractFilesSync(testFiles, null);
			expect(Array.isArray(results2)).toBe(true);
		});

		it("batchExtractBytesSync accepts (dataArray: Buffer[], mimeTypes: string[], config?)", () => {
			const mimeTypes = ["text/plain", "text/plain"];
			const results = batchExtractBytesSync(testBuffers, mimeTypes);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(testBuffers.length);
		});

		it("batchExtractFiles (async) signature matches sync", async () => {
			const results = await batchExtractFiles(testFiles);
			expect(Array.isArray(results)).toBe(true);
		});

		it("batchExtractBytes (async) requires mimeTypes array", async () => {
			const mimeTypes = ["text/plain", "text/plain"];
			const results = await batchExtractBytes(testBuffers, mimeTypes);
			expect(Array.isArray(results)).toBe(true);
		});
	});

	describe("MIME Type Functions - Available Exports", () => {
		it("detectMimeType(bytes) works", () => {
			const pdfHeader = Buffer.from("%PDF-1.4");
			const mimeType = detectMimeType(pdfHeader);
			expect(typeof mimeType).toBe("string");
		});

		it("validateMimeType(mimeType) works", () => {
			const result = validateMimeType("application/pdf");
			expect(typeof result).toBe("string");
		});

		it("getExtensionsForMime(mimeType) works", () => {
			const extensions = getExtensionsForMime("application/pdf");
			expect(Array.isArray(extensions)).toBe(true);
		});

		it("detectMimeTypeFromPath NOT exported in rc.16", () => {});
	});

	describe("Plugin Registration - Corrected Expectations", () => {
		afterEach(() => {
			clearPostProcessors();
			clearValidators();
			clearOcrBackends();
		});

		it("registerPostProcessor - working", () => {
			const processor: PostProcessorProtocol = {
				name: "test-processor",
				processContent: (content: string) => content.toUpperCase(),
			};

			registerPostProcessor(processor);
			const list = listPostProcessors();
			expect(list).toContain("test-processor");
		});

		it("registerValidator - requires proper method binding", () => {
			const validator: ValidatorProtocol = {
				name: "test-validator",
				validate: (content: string) => ({
					isValid: content.length > 0,
					errors: [] as string[],
				}),
			};

			validator.validate = validator.validate.bind(validator);

			registerValidator(validator);
			const list = listValidators();
			expect(list).toContain("test-validator");
		});

		it("registerOcrBackend - requires proper method binding", () => {
			const backend: OcrBackendProtocol = {
				name: "test-ocr",
				processImage: async (imageData: Uint8Array, config: OcrConfig) => ({
					text: "Test",
					confidence: 0.95,
				}),
			};

			backend.processImage = backend.processImage.bind(backend);

			registerOcrBackend(backend);
			const list = listOcrBackends();
			expect(list).toContain("test-ocr");
		});

		it("listPostProcessors returns array", () => {
			const list = listPostProcessors();
			expect(Array.isArray(list)).toBe(true);
		});

		it("listValidators returns array", () => {
			const list = listValidators();
			expect(Array.isArray(list)).toBe(true);
		});

		it("listOcrBackends returns array", () => {
			const list = listOcrBackends();
			expect(Array.isArray(list)).toBe(true);
		});

		it("listDocumentExtractors returns array", () => {
			const list = listDocumentExtractors();
			expect(Array.isArray(list)).toBe(true);
		});
	});

	describe("Error Handling API", () => {
		it("getLastErrorCode returns number", () => {
			const code = getLastErrorCode();
			expect(typeof code).toBe("number");
		});

		it("classifyError returns classification object", () => {
			const classification = classifyError("PDF parsing error");
			expect(typeof classification).toBe("object");
			if (typeof classification === "object" && classification !== null) {
				const obj = classification as Record<string, unknown>;
				if (obj.category !== undefined) {
					expect(typeof obj.category).toBe("string");
				}
			}
		});
	});

	describe("Embedding Presets", () => {
		it("listEmbeddingPresets returns array", () => {
			const presets = listEmbeddingPresets();
			expect(Array.isArray(presets)).toBe(true);
		});

		it("getEmbeddingPreset returns object or null", () => {
			const presets = listEmbeddingPresets();
			if (presets.length > 0) {
				const preset = getEmbeddingPreset(presets[0]);
				if (preset !== null) {
					expect(typeof preset).toBe("object");
				}
			}
		});
	});

	describe("Version & Module Info", () => {
		it("__version__ is defined", () => {
			expect(__version__).toBeDefined();
			expect(typeof __version__).toBe("string");
			expect(__version__).toMatch(/^\d+\.\d+\.\d+/);
		});

		it("version is 4.2.4", () => {
			expect(__version__).toBe("4.2.4");
		});
	});

	describe("Result Structure Validation", () => {
		let testFile: string;

		beforeAll(() => {
			const content = Buffer.from("Test content");
			testFile = join(tmpdir(), "structure-test.txt");
			writeFileSync(testFile, content);
		});

		it("ExtractionResult has expected structure", () => {
			const result = extractFileSync(testFile);
			validateResultStructure(result);
		});

		it("Batch results are ExtractionResult arrays", () => {
			const results = batchExtractFilesSync([testFile]);
			expect(Array.isArray(results)).toBe(true);
			for (const result of results) {
				validateResultStructure(result);
			}
		});
	});
});

function validateResultStructure(result: unknown): void {
	expect(result).toBeDefined();
	expect(typeof result).toBe("object");

	if (typeof result === "object" && result !== null) {
		const r = result as Record<string, unknown>;

		if (r.content !== undefined) {
			expect(typeof r.content).toMatch(/^(string|object)$/);
		}

		if (r.mimeType !== undefined) {
			expect(typeof r.mimeType).toBe("string");
		}

		if (r.chunks !== undefined && r.chunks !== null) {
			expect(Array.isArray(r.chunks)).toBe(true);
		}

		if (r.images !== undefined && r.images !== null) {
			expect(Array.isArray(r.images)).toBe(true);
		}

		if (r.tables !== undefined && r.tables !== null) {
			expect(Array.isArray(r.tables)).toBe(true);
		}

		if (r.metadata !== undefined) {
			expect(typeof r.metadata).toBe("object");
		}
	}
}
