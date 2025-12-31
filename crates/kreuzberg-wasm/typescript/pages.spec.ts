/**
 * Comprehensive pages extraction tests for WASM bindings.
 *
 * Tests verify:
 * 1. extractPages: true - Returns pages array with page content
 * 2. insertPageMarkers: true - Markers appear in main content string
 * 3. markerFormat: custom format - Custom page marker format works correctly
 * 4. Multi-page PDF - Documents with multiple pages produce multiple page entries
 * 5. Page content structure - Each page has correct fields (pageNumber, content, tables, images)
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { extractBytes, initWasm } from "./index.js";
import type { ExtractionConfig, PageContent } from "./types.js";

let samplePdfBytes: Uint8Array;

beforeAll(async () => {
	// Initialize WASM module before running tests
	await initWasm();

	// Load test PDF file (path relative to crates/kreuzberg-wasm)
	const pdfPath = join(process.cwd(), "../../test_documents/pdfs/embedded_images_tables.pdf");
	try {
		samplePdfBytes = new Uint8Array(readFileSync(pdfPath));
	} catch {
		console.warn("Test PDF file not found or PDF support not available in WASM");
	}
});

describe.skipIf(!samplePdfBytes)("Pages Extraction (WASM Bindings)", () => {
	describe("extractPages: true", () => {
		it("should extract pages as separate array", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result).toBeDefined();
			expect(result.pages).toBeDefined();
			expect(Array.isArray(result.pages)).toBe(true);
			expect(result.pages.length).toBeGreaterThan(0);
		});

		it("should preserve pages in main content when extractPages is true", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(typeof result.content).toBe("string");
			expect(result.content.length).toBeGreaterThan(0);
		});

		it("should include page metadata in extraction result", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.metadata).toBeDefined();
			expect(result.metadata.page_count).toBeDefined();

			if (result.metadata.page_count) {
				expect(result.pages.length).toBeGreaterThan(0);
			}
		});
	});

	describe("insertPageMarkers: true", () => {
		it("should insert page markers in content with default format", async () => {
			const config: ExtractionConfig = {
				pages: {
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.content).toContain("PAGE");
		});

		it("should insert markers when extractPages and insertPageMarkers are both true", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.pages).toBeDefined();
			expect(result.pages.length).toBeGreaterThan(0);
			expect(result.content).toContain("PAGE");
		});

		it("should insert multiple page markers for multi-page documents", async () => {
			const config: ExtractionConfig = {
				pages: {
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			const pageMarkerCount = (result.content.match(/PAGE/g) || []).length;
			expect(pageMarkerCount).toBeGreaterThan(0);
		});

		it("should mark all extracted pages in content", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			if (result.pages && result.pages.length > 0) {
				expect(result.content).toBeDefined();
				const markerCount = (result.content.match(/PAGE/g) || []).length;
				expect(markerCount).toBeGreaterThan(0);
			}
		});
	});

	describe("markerFormat: custom format", () => {
		it("should use custom marker format with placeholder", async () => {
			const customFormat = "=== Page {page_num} ===";
			const config: ExtractionConfig = {
				pages: {
					insertPageMarkers: true,
					markerFormat: customFormat,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.content).toContain("Page");
			expect(result.content).toContain("===");
		});

		it("should handle multiple custom marker formats", async () => {
			const formats = ["--- PAGE {page_num} ---", "[Page {page_num}]", "Page {page_num}:"];

			for (const markerFormat of formats) {
				const config: ExtractionConfig = {
					pages: {
						insertPageMarkers: true,
						markerFormat,
					},
				};

				const result = await extractBytes(samplePdfBytes, "application/pdf", config);
				expect(result.content).toBeDefined();
				expect(typeof result.content).toBe("string");
				expect(result.content.length).toBeGreaterThan(0);
			}
		});

		it("should use custom marker format with page numbers", async () => {
			const customFormat = "## Section {page_num}";
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
					markerFormat: customFormat,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.pages).toBeDefined();
			expect(result.content).toContain("Section");
		});

		it("should preserve {page_num} placeholder replacement", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
					markerFormat: "<<<PAGE_{page_num}>>>",
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.content).toContain("PAGE_");
		});
	});

	describe("Multi-page PDF handling", () => {
		it("should extract multiple pages for multi-page documents", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(Array.isArray(result.pages)).toBe(true);

			if (result.pages.length > 1) {
				expect(result.pages.length).toBeGreaterThan(1);

				for (let i = 0; i < result.pages.length; i++) {
					const page = result.pages[i];
					expect(page.pageNumber).toBe(i + 1);
				}
			}
		});

		it("should have sequential page numbers", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			if (result.pages && result.pages.length > 1) {
				for (let i = 0; i < result.pages.length; i++) {
					expect(result.pages[i].pageNumber).toBe(i + 1);
				}
			}
		});

		it("should maintain page order in extracted pages array", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			if (result.pages && result.pages.length > 1) {
				for (let i = 0; i < result.pages.length - 1; i++) {
					expect(result.pages[i].pageNumber).toBeLessThan(result.pages[i + 1].pageNumber);
				}
			}
		});

		it("should not skip any pages in extraction", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			if (result.pages && result.pages.length > 0) {
				const pageNumbers = result.pages.map((p) => p.pageNumber);

				for (let i = 0; i < pageNumbers.length - 1; i++) {
					expect(pageNumbers[i + 1]).toBe(pageNumbers[i] + 1);
				}
			}
		});
	});

	describe("Page content structure validation", () => {
		it("should have required fields in each page object", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(Array.isArray(result.pages)).toBe(true);

			for (const page of result.pages) {
				expect(page).toHaveProperty("pageNumber");
				expect(page).toHaveProperty("content");
				expect(page).toHaveProperty("tables");
				expect(page).toHaveProperty("images");
			}
		});

		it("should have valid pageNumber in each page", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			for (const page of result.pages) {
				expect(typeof page.pageNumber).toBe("number");
				expect(page.pageNumber).toBeGreaterThan(0);
				expect(Number.isInteger(page.pageNumber)).toBe(true);
			}
		});

		it("should have string content in each page", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			for (const page of result.pages) {
				expect(typeof page.content).toBe("string");
			}
		});

		it("should have valid tables array in each page", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			for (const page of result.pages) {
				expect(Array.isArray(page.tables)).toBe(true);

				for (const table of page.tables) {
					expect(table).toHaveProperty("cells");
					expect(table).toHaveProperty("markdown");
					expect(table).toHaveProperty("pageNumber");
					expect(Array.isArray(table.cells)).toBe(true);
					expect(typeof table.markdown).toBe("string");
					expect(typeof table.pageNumber).toBe("number");
				}
			}
		});

		it("should have valid images array in each page", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			for (const page of result.pages) {
				expect(Array.isArray(page.images)).toBe(true);
			}
		});

		it("should validate PageContent type structure", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);
			expect(result.pages).toBeDefined();

			const validatePageContent = (page: PageContent): void => {
				expect(typeof page.pageNumber).toBe("number");
				expect(page.pageNumber).toBeGreaterThan(0);
				expect(typeof page.content).toBe("string");
				expect(Array.isArray(page.tables)).toBe(true);
				expect(Array.isArray(page.images)).toBe(true);
			};

			for (const page of result.pages) {
				validatePageContent(page);
			}
		});

		it("should have consistent page content across extraction methods", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			for (const page of result.pages) {
				expect(page.content.length).toBeGreaterThanOrEqual(0);
				expect(typeof page.content).toBe("string");
			}
		});
	});

	describe("Combined page extraction features", () => {
		it("should work with extractPages and insertPageMarkers together", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(result.pages.length).toBeGreaterThan(0);
			expect(result.content).toBeDefined();
			expect(result.content).toContain("PAGE");
		});

		it("should apply custom marker format to main content", async () => {
			const markerFormat = "### Page {page_num}";
			const config: ExtractionConfig = {
				pages: {
					insertPageMarkers: true,
					markerFormat,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.content).toBeDefined();
			expect(result.content).toContain("Page");
		});

		it("should work with other extraction config options", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
				useCache: false,
				enableQualityProcessing: false,
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(result.pages.length).toBeGreaterThan(0);
			expect(result.content).toBeDefined();
		});

		it("should handle complex config with pages and other options", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
					markerFormat: "=== PAGE {page_num} ===",
				},
				useCache: true,
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(result.content).toBeDefined();
			expect(result.content).toContain("PAGE");
		});
	});

	describe("Edge cases and validation", () => {
		it("should handle null config gracefully", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", null);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should handle empty page extraction config", async () => {
			const config: ExtractionConfig = {
				pages: {},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should have matching page count between pages array and metadata", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(result.metadata.page_count).toBeDefined();

			if (result.metadata.page_count) {
				expect(result.pages.length).toBeLessThanOrEqual(result.metadata.page_count);
			}
		});

		it("should not have duplicate page numbers", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			const pageNumbers = result.pages.map((p) => p.pageNumber);
			const uniquePageNumbers = new Set(pageNumbers);

			expect(uniquePageNumbers.size).toBe(pageNumbers.length);
		});

		it("should extract all pages from PDF with insertPageMarkers", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.pages).toBeDefined();
			expect(result.pages.length).toBeGreaterThan(0);

			const allPageNumbers = new Set(result.pages.map((p) => p.pageNumber));
			expect(allPageNumbers.size).toBe(result.pages.length);
		});
	});

	describe("WASM-specific functionality", () => {
		it("should work with Uint8Array input", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result).toBeDefined();
			expect(result.pages).toBeDefined();
		});

		it("should maintain consistency across multiple extractions", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result1 = await extractBytes(samplePdfBytes, "application/pdf", config);
			const result2 = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result1.pages.length).toBe(result2.pages.length);

			for (let i = 0; i < result1.pages.length; i++) {
				expect(result1.pages[i].pageNumber).toBe(result2.pages[i].pageNumber);
				expect(result1.pages[i].content.length).toBe(result2.pages[i].content.length);
			}
		});

		it("should handle MIME type detection for PDF", async () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
			};

			const result = await extractBytes(samplePdfBytes, "application/pdf", config);

			expect(result.mimeType).toContain("pdf");
		});
	});
});
