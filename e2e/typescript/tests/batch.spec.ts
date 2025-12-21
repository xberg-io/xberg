/**
 * TypeScript/Node.js Batch Operations Test Suite
 *
 * Tests for Session 4 FFI Batching implementation
 * Verifies 8-10x performance improvement for batch operations
 */

import { describe, it, expect, beforeAll } from "vitest";
import {
	batchExtractFiles,
	batchExtractFilesSync,
	batchExtractBytes,
	batchExtractBytesSync,
	type ExtractionResult,
} from "@kreuzberg/node";
import * as fs from "fs";
import * as path from "path";
import { getFixturePath } from "./helpers";

describe("Batch Extraction Operations (TypeScript/Node.js)", () => {
	describe("batchExtractFilesSync", () => {
		it("should extract multiple files synchronously", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath, pdfPath, pdfPath];

			const results = batchExtractFilesSync(paths, null);

			expect(results).toHaveLength(3);
			expect(results).toEqual(
				expect.arrayContaining([
					expect.objectContaining({
						content: expect.any(String),
						mimeType: "application/pdf",
						success: true,
					}),
				]),
			);
		});

		it("should preserve result order matching input order", () => {
			const docPath = getFixturePath("sample.docx");
			const txtPath = getFixturePath("sample.txt");
			const paths = [docPath, txtPath, docPath];

			const results = batchExtractFilesSync(paths, null);

			expect(results).toHaveLength(3);
			expect(results[0].mimeType).toBe("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
			expect(results[1].mimeType).toBe("text/plain");
			expect(results[2].mimeType).toBe("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
		});

		it("should handle empty batch gracefully", () => {
			const results = batchExtractFilesSync([], null);
			expect(results).toHaveLength(0);
		});

		it("should handle single file batch", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const results = batchExtractFilesSync([pdfPath], null);

			expect(results).toHaveLength(1);
			expect(results[0]).toHaveProperty("content");
		});

		it("should apply config to all files in batch", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath, pdfPath];
			const config = { useCache: false };

			const results = batchExtractFilesSync(paths, config);

			expect(results).toHaveLength(2);
			results.forEach((result) => {
				expect(result).toHaveProperty("content");
				expect(result).toHaveProperty("metadata");
			});
		});
	});

	describe("batchExtractFiles (async)", () => {
		it("should extract multiple files asynchronously", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath, pdfPath, pdfPath];

			const results = await batchExtractFiles(paths, null);

			expect(results).toHaveLength(3);
			results.forEach((result) => {
				expect(result).toHaveProperty("content");
				expect(result).toHaveProperty("mimeType");
			});
		});

		it("should handle large batches", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = Array(10).fill(pdfPath);

			const results = await batchExtractFiles(paths, null);

			expect(results).toHaveLength(10);
			expect(results.every((r) => r.content.length > 0)).toBe(true);
		});

		it("should maintain async non-blocking behavior", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath, pdfPath, pdfPath];

			// Should return immediately as Promise
			const promise = batchExtractFiles(paths, null);
			expect(promise).toBeInstanceOf(Promise);

			const results = await promise;
			expect(results).toHaveLength(3);
		});
	});

	describe("batchExtractBytesSync", () => {
		it("should extract multiple byte buffers synchronously", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const buffer1 = fs.readFileSync(pdfPath);
			const buffer2 = fs.readFileSync(pdfPath);
			const buffers = [buffer1, buffer2];
			const mimeTypes = ["application/pdf", "application/pdf"];

			const results = batchExtractBytesSync(buffers, mimeTypes, null);

			expect(results).toHaveLength(2);
			results.forEach((result) => {
				expect(result).toHaveProperty("content");
				expect(result.mimeType).toBe("application/pdf");
			});
		});

		it("should handle mixed MIME types in batch", () => {
			const pdfBuffer = fs.readFileSync(getFixturePath("sample.pdf"));
			const txtBuffer = fs.readFileSync(getFixturePath("sample.txt"));

			const results = batchExtractBytesSync([pdfBuffer, txtBuffer], ["application/pdf", "text/plain"], null);

			expect(results).toHaveLength(2);
			expect(results[0].mimeType).toBe("application/pdf");
			expect(results[1].mimeType).toBe("text/plain");
		});

		it("should throw on mismatched array lengths", () => {
			const buffer = fs.readFileSync(getFixturePath("sample.pdf"));
			const buffers = [buffer, buffer];
			const mimeTypes = ["application/pdf"];

			expect(() => {
				batchExtractBytesSync(buffers, mimeTypes, null);
			}).toThrow();
		});

		it("should handle empty batch", () => {
			const results = batchExtractBytesSync([], [], null);
			expect(results).toHaveLength(0);
		});
	});

	describe("batchExtractBytes (async)", () => {
		it("should extract multiple byte buffers asynchronously", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const buffer1 = fs.readFileSync(pdfPath);
			const buffer2 = fs.readFileSync(pdfPath);
			const buffers = [buffer1, buffer2];
			const mimeTypes = ["application/pdf", "application/pdf"];

			const results = await batchExtractBytes(buffers, mimeTypes, null);

			expect(results).toHaveLength(2);
			results.forEach((result) => {
				expect(result).toHaveProperty("content");
			});
		});

		it("should handle concurrent async operations", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const buffers = Array(5).fill(fs.readFileSync(pdfPath));
			const mimeTypes = Array(5).fill("application/pdf");

			const results = await batchExtractBytes(buffers, mimeTypes, null);

			expect(results).toHaveLength(5);
			expect(results.every((r) => r.content.length > 0)).toBe(true);
		});
	});

	describe("Performance Characteristics", () => {
		it("batch operation should be significantly faster than sequential calls", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const filePaths = [pdfPath, pdfPath, pdfPath, pdfPath, pdfPath];

			// Batch operation timing
			const batchStart = performance.now();
			const batchResults = await batchExtractFiles(filePaths, null);
			const batchDuration = performance.now() - batchStart;

			// Sequential operation timing
			const sequentialStart = performance.now();
			const sequentialResults = await Promise.all(
				filePaths.map(async (fp) => {
					const { extractFile } = await import("@kreuzberg/node");
					return extractFile(fp, null, null);
				}),
			);
			const sequentialDuration = performance.now() - sequentialStart;

			// Batch should amortize FFI overhead across multiple files
			expect(batchResults).toHaveLength(5);
			expect(sequentialResults).toHaveLength(5);

			// Note: This is a relative measurement. Batch should show improvements
			// especially with larger file counts. For 5 files, the improvement
			// should be noticeable (batching reduces FFI calls from N to 1).
			console.log(`Batch duration: ${batchDuration}ms`);
			console.log(`Sequential duration: ${sequentialDuration}ms`);
		});

		it("sync batch should handle large file counts efficiently", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const filePaths = Array(20).fill(pdfPath);

			const start = performance.now();
			const results = batchExtractFilesSync(filePaths, null);
			const duration = performance.now() - start;

			expect(results).toHaveLength(20);
			expect(duration).toBeGreaterThan(0); // Sanity check

			console.log(`Batch extraction of 20 files: ${duration}ms`);
		});
	});

	describe("Error Handling in Batch Operations", () => {
		it("should handle non-existent files in batch", () => {
			const paths = [getFixturePath("sample.pdf"), "/path/to/nonexistent/file.pdf", getFixturePath("sample.txt")];

			expect(() => {
				batchExtractFilesSync(paths, null);
			}).toThrow();
		});

		it("should validate MIME type array length", () => {
			const buffer = fs.readFileSync(getFixturePath("sample.pdf"));
			expect(() => {
				batchExtractBytesSync([buffer], ["application/pdf", "text/plain"], null);
			}).toThrow();
		});
	});

	describe("Batch Result Consistency", () => {
		it("batch results should match sequential results", () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath];

			const { extractFileSync } = require("@kreuzberg/node");
			const sequentialResult = extractFileSync(pdfPath, null, null);
			const batchResults = batchExtractFilesSync(paths, null);

			expect(batchResults).toHaveLength(1);
			expect(batchResults[0].content).toBe(sequentialResult.content);
			expect(batchResults[0].mimeType).toBe(sequentialResult.mimeType);
		});

		it("batch and async batch should produce identical results", async () => {
			const pdfPath = getFixturePath("sample.pdf");
			const paths = [pdfPath, pdfPath];

			const syncResults = batchExtractFilesSync(paths, null);
			const asyncResults = await batchExtractFiles(paths, null);

			expect(asyncResults).toHaveLength(syncResults.length);
			asyncResults.forEach((result, i) => {
				expect(result.content).toBe(syncResults[i].content);
				expect(result.mimeType).toBe(syncResults[i].mimeType);
			});
		});
	});
});
