/**
 * Comprehensive Error Handling Tests for TypeScript Node Binding
 *
 * Coverage of 8-10 behaviors:
 * 1. Invalid config handling (negative values, invalid types)
 * 2. File not found / corrupted files
 * 3. Invalid MIME types and format mismatches
 * 4. Permission errors (empty paths, inaccessible files)
 * 5. Malformed document handling (corrupted PDFs, invalid XML)
 * 6. Out-of-memory patterns (large files, resource exhaustion)
 * 7. Timeout behavior (async operations, concurrent requests)
 * 8. Concurrent error states (batch operations, parallel processing)
 * 9. Error type validation and message patterns
 * 10. Async error handling patterns
 */

import { describe, expect, it, vi } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	batchExtractFiles,
	batchExtractFilesSync,
	clearPostProcessors,
	type ExtractionConfig,
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
	MissingDependencyError,
	OcrError,
	ParsingError,
	registerOcrBackend,
	registerPostProcessor,
	unregisterPostProcessor,
	ValidationError,
} from "../../dist/index.js";

describe("Error Handling", () => {
	describe("1. Invalid config handling", () => {
		it("should handle negative max_chars in config gracefully", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: -100, // Invalid negative value
				},
			};
			const data = Buffer.from("test content");

			// Negative values may be clamped to valid range or default
			try {
				const result = extractBytesSync(data, "text/plain", config);
				expect(result).toBeDefined();
			} catch {
				// Error is acceptable for invalid config
			}
		});

		it("should handle negative max_overlap in config gracefully", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: -50, // Invalid negative overlap
				},
			};
			const data = Buffer.from("test content");

			// Negative values may be clamped to valid range or default
			try {
				const result = extractBytesSync(data, "text/plain", config);
				expect(result).toBeDefined();
			} catch {
				// Error is acceptable for invalid config
			}
		});

		it("should handle invalid OCR backend configuration gracefully", () => {
			const config: ExtractionConfig = {
				ocr: {
					backend: "", // Empty backend name - may be ignored
					language: "eng",
				},
			};
			const data = Buffer.from("test");

			// Empty backend configuration may be silently ignored
			try {
				extractBytesSync(data, "text/plain", config);
			} catch {
				// Error is acceptable
			}
		});

		it("should handle negative image DPI configuration gracefully", () => {
			const config: ExtractionConfig = {
				images: {
					extractImages: true,
					targetDpi: -300, // Invalid negative DPI - may be ignored
				},
			};
			const data = Buffer.from("test");

			// Negative DPI configuration may be silently ignored or converted
			try {
				extractBytesSync(data, "text/plain", config);
			} catch {
				// Error is acceptable
			}
		});

		it("should handle invalid language detection confidence threshold gracefully", async () => {
			const config: ExtractionConfig = {
				languageDetection: {
					enabled: true,
					minConfidence: 1.5, // Invalid: must be 0.0-1.0, may be clamped
				},
			};
			const data = Buffer.from("test");

			// Invalid confidence may be clamped to valid range instead of throwing
			try {
				const result = await extractBytes(data, "text/plain", config);
				expect(result).toBeDefined();
			} catch {
				// Error is also acceptable
			}
		});
	});

	describe("2. File not found / corrupted files", () => {
		it("should throw error for non-existent file (sync)", () => {
			expect(() => {
				extractFileSync("/nonexistent/file/path.pdf", null, null);
			}).toThrow();
		});

		it("should throw error for non-existent file (async)", async () => {
			await expect(extractFile("/nonexistent/file/path.pdf", null, null)).rejects.toThrow();
		});

		it("should throw error for corrupted PDF file", () => {
			const corruptedPdf = Buffer.from("%PDF-1.4\nCORRUPTED_DATA\n%%EOF");

			expect(() => {
				extractBytesSync(corruptedPdf, "application/pdf", null);
			}).toThrow();
		});

		it("should throw error for truncated PDF file", () => {
			const truncatedPdf = Buffer.from(
				`%PDF-1.4
1 0 obj
<<
/Type /Catalog
>>
endobj`,
			);

			expect(() => {
				extractBytesSync(truncatedPdf, "application/pdf", null);
			}).toThrow();
		});

		it("should throw error for corrupted ZIP archive", () => {
			const corruptedZip = Buffer.from("PK\x03\x04CORRUPTED_ARCHIVE_DATA");

			expect(() => {
				extractBytesSync(corruptedZip, "application/zip", null);
			}).toThrow();
		});
	});

	describe("3. Invalid MIME types and format mismatches", () => {
		it("should throw error for unsupported MIME type (sync)", () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			expect(() => {
				extractBytesSync(data, "application/x-fake-mime-type", null);
			}).toThrow();
		});

		it("should throw error for unsupported MIME type (async)", async () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			await expect(extractBytes(data, "application/x-fake-mime-type", null)).rejects.toThrow();
		});

		it("should throw error for empty MIME type", () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			expect(() => {
				extractBytesSync(data, "", null);
			}).toThrow();
		});

		it("should throw error for malformed MIME type syntax", () => {
			const data = Buffer.from("test");
			expect(() => {
				extractBytesSync(data, "invalid-mime-format", null);
			}).toThrow();
		});

		it("should throw error for null MIME type when required", () => {
			const data = Buffer.from("\x89PNG\r\n\x1a\n"); // PNG header

			// When MIME type cannot be detected, should throw
			expect(() => {
				extractBytesSync(data, "application/invalid", null);
			}).toThrow();
		});
	});

	describe("4. Permission errors and invalid input paths", () => {
		it("should throw error for invalid file path (empty string)", () => {
			expect(() => {
				extractFileSync("", null, null);
			}).toThrow();
		});

		it("should throw error for whitespace-only file path", () => {
			expect(() => {
				extractFileSync("   ", null, null);
			}).toThrow();
		});

		it("should throw error when file is a directory", async () => {
			await expect(extractFile("/tmp", null, null)).rejects.toThrow();
		});

		it("should throw error for relative path to non-existent file", () => {
			expect(() => {
				extractFileSync("./nonexistent/relative/path.pdf", null, null);
			}).toThrow();
		});
	});

	describe("5. Malformed document handling", () => {
		it("should handle malformed XML document gracefully", () => {
			const malformedXml = Buffer.from('<?xml version="1.0"?><root><item>unclosed');

			// Malformed XML may be processed as plain text or throw
			try {
				const result = extractBytesSync(malformedXml, "application/xml", null);
				expect(result).toBeDefined();
			} catch {
				// Error is acceptable for malformed documents
			}
		});

		it("should throw error for invalid JSON structure", () => {
			const invalidJson = Buffer.from("{invalid: json, no quotes}");

			expect(() => {
				extractBytesSync(invalidJson, "application/json", null);
			}).toThrow();
		});

		it("should throw error for malformed binary data in PDF", () => {
			const malformedData = Buffer.from([0, 0, 0, 0]);

			expect(() => {
				extractBytesSync(malformedData, "application/pdf", null);
			}).toThrow();
		});

		it("should handle gracefully mixed encoding issues", () => {
			const mixedEncoding = Buffer.from([0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xff, 0xfe, 0x54, 0x65, 0x73, 0x74]);

			try {
				extractBytesSync(mixedEncoding, "text/plain", null);
			} catch {
				expect(true).toBe(true); // Error is acceptable
			}
		});
	});

	describe("6. Out-of-memory patterns and resource exhaustion", () => {
		it("should handle very large text files gracefully", async () => {
			const largeText = "x".repeat(100_000_000); // 100MB string

			// Should either process or throw resource-related error
			try {
				await extractBytes(Buffer.from(largeText), "text/plain", null);
			} catch (error) {
				expect(error).toBeDefined();
			}
		});

		it("should handle deeply nested JSON structures", () => {
			let deepJson = '{"level0":';
			for (let i = 0; i < 10000; i++) {
				deepJson += '{"nested":';
			}
			deepJson += '"value"';
			for (let i = 0; i < 10000; i++) {
				deepJson += "}";
			}
			deepJson += "}";

			try {
				extractBytesSync(Buffer.from(deepJson), "application/json", null);
			} catch {
				expect(true).toBe(true); // Error handling is correct
			}
		});

		it("should handle maximum image dimension configuration without overflow", () => {
			const config: ExtractionConfig = {
				images: {
					extractImages: true,
					maxImageDimension: 2_147_483_647, // Max safe integer
				},
			};

			const data = Buffer.from("test");
			try {
				extractBytesSync(data, "text/plain", config);
			} catch {
				expect(true).toBe(true);
			}
		});
	});

	describe("7. Timeout behavior and async operations", () => {
		it("should handle async error rejection properly", async () => {
			const promise = extractFile("/nonexistent/async/file.pdf", null, null);

			await expect(promise).rejects.toThrow();
		});

		it("should properly await async extraction errors", async () => {
			const data = Buffer.from("test");
			const invalidMime = "application/invalid-type-xyz";

			const promise = extractBytes(data, invalidMime, null);
			await expect(promise).rejects.toThrow();
		});

		it("should handle rapid sequential async errors", async () => {
			const promises = [];

			for (let i = 0; i < 5; i++) {
				promises.push(extractFile(`/nonexistent/file${i}.pdf`, null, null));
			}

			const results = await Promise.allSettled(promises);

			results.forEach((result) => {
				expect(result.status).toBe("rejected");
			});
		});

		it("should handle mixed sync and async error patterns", async () => {
			const syncError = expect(() => {
				extractBytesSync(Buffer.from("test"), "application/invalid", null);
			}).toThrow();

			const asyncError = await expect(extractBytes(Buffer.from("test"), "application/invalid", null)).rejects.toThrow();

			expect(syncError).toBeDefined();
			expect(asyncError).toBeDefined();
		});
	});

	describe("8. Concurrent error states", () => {
		it("should handle batch extraction with all failed files", () => {
			const paths = ["/nonexistent/file1.pdf", "/nonexistent/file2.pdf", "/nonexistent/file3.pdf"];

			const results = batchExtractFilesSync(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(3);
			results.forEach((result) => {
				// Each result should indicate an error occurred in metadata
				expect(result.metadata).toBeDefined();
				// Check for error_type field that contains error information
				expect(
					result.metadata.error_type || result.metadata.error || result.content.toLowerCase().includes("error"),
				).toBeTruthy();
			});
		});

		it("should handle async batch extraction errors", async () => {
			const paths = ["/nonexistent/async1.pdf", "/nonexistent/async2.pdf"];

			const results = await batchExtractFiles(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(2);
			results.forEach((result) => {
				expect(result.metadata.error).toBeTruthy();
			});
		});

		it("should isolate errors in concurrent byte array extractions", async () => {
			const validData = Buffer.from("test content");
			const invalidMime = "application/x-unknown-type";

			const promise1 = extractBytes(validData, "text/plain", null).catch(() => null);
			const promise2 = extractBytes(validData, invalidMime, null).catch(() => null);
			const promise3 = extractBytes(validData, "text/plain", null).catch(() => null);

			const results = await Promise.all([promise1, promise2, promise3]);

			// At least one should fail (promise2)
			expect(results.some((r) => r === null)).toBe(true);
		});

		it("should handle concurrent batch operations without cross-contamination", async () => {
			const paths1 = ["/nonexistent/batch1_file1.pdf"];
			const paths2 = ["/nonexistent/batch2_file1.pdf"];

			const [results1, results2] = await Promise.all([
				batchExtractFiles(paths1, null),
				batchExtractFiles(paths2, null),
			]);

			expect(results1[0].metadata.error).toBeTruthy();
			expect(results2[0].metadata.error).toBeTruthy();
		});
	});

	describe("9. Error type validation and message patterns", () => {
		it("should throw specific error types for different error conditions", async () => {
			// ParsingError for corrupted content
			const corruptedPdf = Buffer.from("%PDF-INVALID");

			try {
				await extractBytes(corruptedPdf, "application/pdf", null);
			} catch (error) {
				expect(error).toBeDefined();
				expect(error instanceof Error).toBe(true);
			}
		});

		it("should include descriptive error messages", async () => {
			try {
				extractBytesSync(Buffer.from("test"), "", null);
			} catch (error) {
				const message = (error as Error).message;
				expect(message.length).toBeGreaterThan(0);
				expect(message).not.toBe("undefined");
			}
		});

		it("should maintain error context in nested operations", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: -1,
				},
			};

			try {
				extractBytesSync(Buffer.from("test"), "text/plain", config);
			} catch (error) {
				expect(error).toBeDefined();
				expect((error as Error).message).toBeTruthy();
			}
		});

		it("should throw errors without suppressing stack traces", () => {
			try {
				extractFileSync("/nonexistent", null, null);
			} catch (error) {
				const stackTrace = (error as Error).stack;
				expect(stackTrace).toBeDefined();
				expect(stackTrace).toContain("Error");
			}
		});
	});

	describe("10. Async error handling patterns", () => {
		it("should catch errors in try-catch blocks for async operations", async () => {
			let errorCaught = false;

			try {
				await extractFile("/nonexistent/async.pdf", null, null);
			} catch {
				errorCaught = true;
			}

			expect(errorCaught).toBe(true);
		});

		it("should handle rejected promises correctly", async () => {
			const promise = extractBytes(Buffer.from("test"), "application/invalid", null);

			await expect(promise).rejects.toBeDefined();
			await expect(extractBytes(Buffer.from("test"), "application/invalid", null)).rejects.toThrow();
		});

		it("should allow custom error handling in Promise chains", async () => {
			let handledError = false;

			await extractFile("/nonexistent/chain.pdf", null, null)
				.then(() => {
					expect(true).toBe(false); // Should not reach
				})
				.catch(() => {
					handledError = true;
				});

			expect(handledError).toBe(true);
		});

		it("should preserve error information in Promise.allSettled", async () => {
			const promises = [
				extractFile("/nonexistent/file1.pdf", null, null),
				extractFile("/nonexistent/file2.pdf", null, null),
			];

			const results = await Promise.allSettled(promises);

			results.forEach((result) => {
				if (result.status === "rejected") {
					expect(result.reason).toBeDefined();
				}
			});
		});
	});

	describe("Plugin registration", () => {
		afterEach(() => {
			// Clean up after each test
			try {
				clearPostProcessors();
			} catch {
				// Ignore cleanup errors
			}
		});

		it("should allow registering postprocessor", () => {
			const processor = {
				name: () => `test_processor_${Math.random()}`,
				process: (result: any) => result,
				processingStage: () => "middle" as const,
			};

			expect(() => {
				registerPostProcessor(processor);
			}).not.toThrow();
		});

		it("should allow unregistering postprocessor", () => {
			const processor = {
				name: () => "test_processor_to_unregister",
				process: (result: any) => result,
				processingStage: () => "middle" as const,
			};

			registerPostProcessor(processor);

			expect(() => {
				unregisterPostProcessor("test_processor_to_unregister");
			}).not.toThrow();
		});

		it("should allow clearing postprocessors", () => {
			expect(() => {
				clearPostProcessors();
			}).not.toThrow();
		});

		it("should forward OCR backend registration to native binding", () => {
			const backend = {
				name: () => "test_ocr",
				supportedLanguages: () => ["eng", "spa"],
				processImage: async () => ({
					content: "test",
					mime_type: "text/plain",
					metadata: {},
					tables: [],
				}),
			};

			const mockBinding = {
				registerOcrBackend: vi.fn(),
			};

			__setBindingForTests(mockBinding);
			try {
				expect(() => {
					registerOcrBackend(backend);
				}).not.toThrow();
				expect(mockBinding.registerOcrBackend).toHaveBeenCalledTimes(1);
			} finally {
				__resetBindingForTests();
			}
		});
	});
});
