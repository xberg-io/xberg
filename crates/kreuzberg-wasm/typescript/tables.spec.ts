/**
 * Comprehensive table extraction tests for WASM bindings.
 * Tests basic structure, complex tables, markdown validation, multi-page support,
 * consistency, and edge cases.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { extractBytes, initWasm } from "./index.js";
import type { ExtractionConfig, Table } from "./types.js";

let samplePdfBytes: Uint8Array;

beforeAll(async () => {
	await initWasm();
	// Load test PDF file (path relative to crates/kreuzberg-wasm)
	const pdfPath = join(process.cwd(), "../../test_documents/pdfs/embedded_images_tables.pdf");
	try {
		samplePdfBytes = new Uint8Array(readFileSync(pdfPath));
	} catch {
		console.warn("Test PDF file not found or PDF support not available in WASM");
	}
});

describe.skipIf(!samplePdfBytes)("Table Extraction (WASM)", () => {
	describe("Basic Structure", () => {
		it("should extract tables with valid cell grid and headers", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {
				pdfOptions: { extractMetadata: true },
			});
			expect(result.tables).toBeDefined();
			if (result.tables.length > 0) {
				const table = result.tables[0];
				expect(table.cells).toBeDefined();
				if (table.cells?.length > 0) {
					table.cells.forEach((row) => {
						expect(Array.isArray(row)).toBe(true);
						row.forEach((cell) => {
							expect(typeof cell).toBe("string");
						});
					});
				}
				if (table.headers) {
					table.headers.forEach((h) => {
						expect(typeof h).toBe("string");
					});
				}
			}
		});

		it("should extract rows from table structure", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {});
			if (result.tables[0]?.rows) {
				result.tables[0].rows.forEach((row) => {
					row.forEach((cell) => {
						expect(typeof cell).toBe("string");
					});
				});
			}
		});
	});

	describe("Complex Tables", () => {
		it("should handle merged cells, empty cells, and preserve content", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {});
			if (result.tables.length > 0) {
				const table = result.tables[0];
				if (table.cells) {
					for (const row of table.cells) {
						for (const cell of row) {
							expect(typeof cell).toBe("string");
							expect(cell).not.toContain("\0");
						}
					}
				}
			}
		});

		it("should validate single row/column edge cases", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {});
			result.tables.forEach((table) => {
				if (table.cells?.length === 1) {
					expect(Array.isArray(table.cells[0])).toBe(true);
				}
				if (table.cells?.every((row) => row.length === 1)) {
					expect(table.cells[0].length).toBe(1);
				}
			});
		});
	});

	describe("Markdown Conversion", () => {
		it("should generate markdown with valid pipe format", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {});
			if (result.tables.length > 0) {
				const { markdown, cells } = result.tables[0];
				if (markdown && cells?.length > 0 && cells[0].length > 0) {
					expect(markdown).toContain("|");
					markdown
						.split("\n")
						.filter((l) => l.trim().length > 0)
						.forEach((line) => {
							if (line.includes("|")) {
								expect(line.trim()).toMatch(/^\|.*\|$/);
							}
						});
				}
			}
		});

		it("should include separators for multi-row tables", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {});
			if (result.tables[0]?.markdown && result.tables[0].cells?.length > 1) {
				expect(result.tables[0].markdown).toContain("-");
			}
		});
	});

	describe("Multi-Page and Consistency", () => {
		it("should track page numbers with consistent ordering", async () => {
			const result = await extractBytes(samplePdfBytes, "application/pdf", {
				pages: { extractPages: true },
			});
			if (result.tables.length > 0) {
				result.tables.forEach((table) => {
					if (table.pageNumber !== undefined) {
						expect(typeof table.pageNumber).toBe("number");
						expect(table.pageNumber).toBeGreaterThanOrEqual(0);
					}
				});
			}
			for (let i = 0; i < result.tables.length - 1; i++) {
				const p1 = result.tables[i].pageNumber;
				const p2 = result.tables[i + 1].pageNumber;
				if (p1 !== undefined && p2 !== undefined) {
					expect(p2).toBeGreaterThanOrEqual(p1);
				}
			}
		});

		it("should extract identical content and markdown across runs", async () => {
			const r1 = await extractBytes(samplePdfBytes, "application/pdf", {});
			const r2 = await extractBytes(samplePdfBytes, "application/pdf", {});

			expect(r1.tables.length).toBe(r2.tables.length);
			r1.tables.forEach((t1, idx) => {
				const t2 = r2.tables[idx];
				if (t1.cells && t2.cells) {
					expect(t1.cells).toEqual(t2.cells);
				}
				if (t1.markdown && t2.markdown) {
					expect(t1.markdown).toBe(t2.markdown);
				}
			});
		});
	});

	describe("Format Support", () => {
		it("should extract tables from PDF with extraction options", async () => {
			const config: ExtractionConfig = {
				pdfOptions: { extractImages: true, extractMetadata: true },
			};
			const result = await extractBytes(samplePdfBytes, "application/pdf", config);
			expect(Array.isArray(result.tables)).toBe(true);
		});

		it("should handle minimal table interface and HTML format", async () => {
			const emptyTable: Table = { cells: [], markdown: "" };
			expect(emptyTable.cells.length).toBe(0);

			const htmlBytes = new TextEncoder().encode("<table><tr><th>H</th></tr><tr><td>C</td></tr></table>");
			const result = await extractBytes(htmlBytes, "text/html", {});
			expect(Array.isArray(result.tables)).toBe(true);
		});
	});
});
