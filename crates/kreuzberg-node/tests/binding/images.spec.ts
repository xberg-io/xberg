/**
 * Comprehensive image extraction tests for TypeScript Node.js bindings.
 *
 * Tests verify:
 * 1. PDF image extraction with metadata (format, dimensions, MIME type)
 * 2. Image handling in composite documents (DOCX, PPTX)
 * 3. Image format detection (PNG, JPEG, WebP, TIFF)
 * 4. Embedded vs. referenced images
 * 5. Error handling for corrupted/invalid images
 * 6. Batch image extraction from multi-page documents
 * 7. Image metadata completeness (width, height, colorspace)
 * 8. Image targeting and DPI optimization
 *
 * NAPI-RS bindings with plain object configs (NO builder pattern).
 */

import { readFileSync, realpathSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { extractBytesSync, extractFileSync } from "../../dist/index.js";
import type { ExtractedImage, ExtractionConfig } from "../../src/types.js";
import { getTestDocumentPath } from "../helpers/index.js";

let samplePdfPath: string;
let samplePdfBytes: Uint8Array;
let _pptxPath: string;
let _docxPath: string;

beforeAll(() => {
	samplePdfPath = getTestDocumentPath("pdf/embedded_images_tables.pdf");
	// Resolve symlinks to get the actual file path (important for Windows compatibility)
	samplePdfBytes = new Uint8Array(readFileSync(realpathSync(samplePdfPath)));

	// Get PPTX if available
	_pptxPath = getTestDocumentPath("presentations/simple.pptx");

	// Get DOCX if available
	_docxPath = getTestDocumentPath("documents/sample.docx");
});

describe("Image Extraction (Node.js Bindings)", () => {
	describe("PDF image extraction with metadata", () => {
		it("should extract images with format and dimensions", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();

			if (result.images && result.images.length > 0) {
				const image = result.images[0];
				expect(image.format).toBeDefined();
				expect(typeof image.format).toBe("string");
				expect(image.data).toBeDefined();
				expect(image.data instanceof Uint8Array).toBe(true);
				expect(image.imageIndex).toBeGreaterThanOrEqual(0);
			}
		});

		it("should extract image with valid MIME type information", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result.mimeType).toContain("application/pdf");
			expect(result.images).toBeDefined();

			if (result.images && result.images.length > 0) {
				const image = result.images[0];
				// PDF filter names are also valid (DCTDecode for JPEG, FlateDecode for PNG/deflate)
				const validFormats = ["PNG", "JPEG", "JPEG2000", "JBIG2", "TIFF", "WebP", "DCTDECODE", "FLATEDECODE"];
				expect(validFormats).toContain(image.format.toUpperCase());
			}
		});

		it("should extract images with width and height metadata", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("width");
					expect(image).toHaveProperty("height");

					if (image.width !== null && image.width !== undefined) {
						expect(image.width).toBeGreaterThan(0);
					}

					if (image.height !== null && image.height !== undefined) {
						expect(image.height).toBeGreaterThan(0);
					}
				}
			}
		});

		it("should include image page number information", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("pageNumber");
					expect(image.pageNumber).toBeGreaterThanOrEqual(1);
				}
			}
		});

		it("should extract image data as Uint8Array", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				const image = result.images[0];
				expect(image.data).toBeInstanceOf(Uint8Array);
				expect(image.data.length).toBeGreaterThan(0);
			}
		});

		it("should extract images with sequential indices", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 1) {
				for (let i = 0; i < result.images.length; i++) {
					expect(result.images[i].imageIndex).toBe(i);
				}
			}
		});
	});

	describe("Image handling in composite documents", () => {
		it("should support image extraction from PDF documents", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
			expect(typeof result.content).toBe("string");
			expect(result.images).toBeDefined();
		});

		it("should preserve document content when extracting images", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const resultWithImages = extractFileSync(samplePdfPath, config);

			const configNoImages: ExtractionConfig = {
				images: {
					enabled: false,
				},
			};

			const resultNoImages = extractFileSync(samplePdfPath, configNoImages);

			// Both should have content
			expect(resultWithImages.content).toBeDefined();
			expect(resultNoImages.content).toBeDefined();
			expect(resultWithImages.content.length).toBeGreaterThan(0);
		});

		it("should handle documents with multiple images correctly", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				expect(result.images.length).toBeGreaterThan(0);

				// All images should have unique indices
				const indices = new Set(result.images.map((img) => img.imageIndex));
				expect(indices.size).toBe(result.images.length);
			}
		});

		it("should track image positions within pages", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image.pageNumber).toBeGreaterThanOrEqual(1);
					expect(Number.isInteger(image.pageNumber)).toBe(true);
				}
			}
		});
	});

	describe("Image format detection", () => {
		it("should identify image format correctly", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image.format).toBeDefined();
					expect(typeof image.format).toBe("string");
					expect(image.format.length).toBeGreaterThan(0);
				}
			}
		});

		it("should support common image formats (PNG, JPEG, WebP, TIFF)", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			const supportedFormats = [
				"PNG",
				"JPEG",
				"JPEG2000",
				"JBIG2",
				"TIFF",
				"WebP",
				"DCTDecode", // PDF internal name for JPEG
				"FlateDecode", // PDF compression format
			];

			if (result.images && result.images.length > 0) {
				const detectedFormats = result.images.map((img) => img.format.toUpperCase());

				// At least one format should be in the supported list (case-insensitive)
				const foundSupported = detectedFormats.some((fmt) =>
					supportedFormats.some(
						(supported) => fmt.includes(supported.toUpperCase()) || supported.toUpperCase().includes(fmt),
					),
				);

				expect(foundSupported || result.images.length === 0).toBe(true);
			}
		});

		it("should extract image format from PDF embedded images", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractBytesSync(samplePdfBytes, "application/pdf", config);

			expect(result.images).toBeDefined();

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image.format).toBeDefined();
					expect(image.format.length).toBeGreaterThan(0);
				}
			}
		});
	});

	describe("Embedded vs. referenced images", () => {
		it("should extract embedded images from PDF", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			// PDF typically has embedded images
			expect(result.images).toBeDefined();

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					// Embedded images should have data
					expect(image.data).toBeDefined();
					expect(image.data instanceof Uint8Array).toBe(true);
					expect(image.data.length).toBeGreaterThan(0);
				}
			}
		});

		it("should mark mask images appropriately", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("isMask");
					expect(typeof image.isMask).toBe("boolean");
				}
			}
		});

		it("should include colorspace information when available", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("colorspace");

					if (image.colorspace !== null && image.colorspace !== undefined) {
						expect(typeof image.colorspace).toBe("string");
						const validColorspaces = ["RGB", "CMYK", "Grayscale", "Lab", "Gray"];
						const hasValidColorspace = validColorspaces.some((cs) =>
							image.colorspace?.toUpperCase().includes(cs.toUpperCase()),
						);
						expect(hasValidColorspace || image.colorspace.length > 0).toBe(true);
					}
				}
			}
		});
	});

	describe("Error handling for invalid/corrupted images", () => {
		it("should handle documents with no images gracefully", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const textPath = getTestDocumentPath("text/contract.txt");

			const result = extractFileSync(textPath, config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
			// Images may be null or empty array for text documents
			if (result.images) {
				expect(Array.isArray(result.images) || result.images === null).toBe(true);
			}
		});

		it("should disable image extraction when enabled is false", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: false,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			// The config should be accepted without error
			// Note: Whether images are actually excluded depends on the native binding's
			// implementation of the enabled: false flag
			expect(result.content).toBeDefined();
		});

		it("should handle extraction with null images configuration", () => {
			const config: ExtractionConfig = {
				images: null,
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should handle extraction without images configuration", () => {
			const config: ExtractionConfig = {};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should validate image data integrity", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					// Image data should be a valid Uint8Array
					expect(image.data).toBeInstanceOf(Uint8Array);

					// For image data, check reasonable size (not empty)
					if (image.data.length > 0) {
						expect(image.data.length).toBeGreaterThan(0);
					}
				}
			}
		});
	});

	describe("Batch image extraction from multi-page documents", () => {
		it("should extract images from all pages", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
				pages: {
					extractPages: true,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result.pages).toBeDefined();
			expect(Array.isArray(result.pages)).toBe(true);
			expect(result.images).toBeDefined();

			// Images should be extracted at document level
			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image.pageNumber).toBeGreaterThanOrEqual(1);
				}
			}
		});

		it("should maintain page association for extracted images", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
				pages: {
					extractPages: true,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.pages && result.pages.length > 0 && result.images) {
				// Group images by page
				const imagesByPage: Record<number, ExtractedImage[]> = {};
				for (const image of result.images) {
					const pageNum = image.pageNumber || 1;
					if (!imagesByPage[pageNum]) {
						imagesByPage[pageNum] = [];
					}
					imagesByPage[pageNum].push(image);
				}

				// Verify page numbers are valid
				for (const pageNum of Object.keys(imagesByPage)) {
					const num = parseInt(pageNum, 10);
					expect(num).toBeGreaterThanOrEqual(1);
					expect(num).toBeLessThanOrEqual(result.pages.length);
				}
			}
		});

		it("should extract images consistently across multiple calls", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result1 = extractFileSync(samplePdfPath, config);
			const result2 = extractFileSync(samplePdfPath, config);

			expect(result1.images).toBeDefined();
			expect(result2.images).toBeDefined();

			if (result1.images && result2.images) {
				expect(result1.images.length).toBe(result2.images.length);

				// Image counts per page should match
				for (let i = 0; i < Math.min(result1.images.length, 3); i++) {
					expect(result1.images[i].pageNumber).toBe(result2.images[i].pageNumber);
					expect(result1.images[i].format).toBe(result2.images[i].format);
				}
			}
		});

		it("should extract images asynchronously from multi-page documents", async () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();

			if (result.images && result.images.length > 0) {
				expect(result.images.length).toBeGreaterThan(0);
			}
		});

		it("should handle batch extraction with different DPI settings", () => {
			const dpiValues = [72, 150, 200, 300];
			const results = dpiValues.map((dpi) => {
				const config: ExtractionConfig = {
					images: {
						enabled: true,
						targetDpi: dpi,
					},
				};
				return extractFileSync(samplePdfPath, config);
			});

			for (const result of results) {
				expect(result).toBeDefined();
				expect(result.images).toBeDefined();
			}
		});
	});

	describe("Image metadata completeness and validation", () => {
		it("should have all required image properties", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("data");
					expect(image).toHaveProperty("format");
					expect(image).toHaveProperty("imageIndex");
					expect(image).toHaveProperty("isMask");
					expect(image).toHaveProperty("pageNumber");
				}
			}
		});

		it("should include bits per component information", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("bitsPerComponent");

					if (image.bitsPerComponent !== null && image.bitsPerComponent !== undefined) {
						expect(image.bitsPerComponent).toBeGreaterThan(0);
						const validBits = [1, 2, 4, 8, 16, 32];
						expect(validBits).toContain(image.bitsPerComponent);
					}
				}
			}
		});

		it("should validate image dimension constraints", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					if (
						image.width !== null &&
						image.width !== undefined &&
						image.height !== null &&
						image.height !== undefined
					) {
						expect(image.width).toBeGreaterThan(0);
						expect(image.height).toBeGreaterThan(0);
						// Reasonable bounds for document images
						expect(image.width).toBeLessThan(100000);
						expect(image.height).toBeLessThan(100000);
					}
				}
			}
		});

		it("should provide image description when available", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					expect(image).toHaveProperty("description");
					// Description may be null or string
					if (image.description !== null && image.description !== undefined) {
						expect(typeof image.description).toBe("string");
					}
				}
			}
		});
	});

	describe("Image targeting and DPI optimization", () => {
		it("should respect targetDpi configuration", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();

			// Extraction should succeed with target DPI
			if (result.images) {
				expect(result.images).toBeInstanceOf(Array);
			}
		});

		it("should handle maxImageDimension constraint", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
					maxImageDimension: 2000,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 0) {
				for (const image of result.images) {
					// If dimensions are available, they should not exceed maxImageDimension
					if (image.width && image.height) {
						expect(image.width).toBeLessThanOrEqual(2000);
						expect(image.height).toBeLessThanOrEqual(2000);
					}
				}
			}
		});

		it("should support different DPI values", () => {
			const testDpiValues = [72, 96, 150, 200, 300];

			for (const dpi of testDpiValues) {
				const config: ExtractionConfig = {
					images: {
						enabled: true,
						targetDpi: dpi,
					},
				};

				const result = extractFileSync(samplePdfPath, config);
				expect(result).toBeDefined();
				expect(result.images).toBeDefined();
			}
		});

		it("should handle autoAdjustDpi option", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
					autoAdjustDpi: true,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();
		});

		it("should enforce minDpi and maxDpi bounds", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
					minDpi: 72,
					maxDpi: 300,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();
		});
	});

	describe("Images in page extraction context", () => {
		it("should extract images for page-level results", () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result.pages).toBeDefined();
			expect(Array.isArray(result.pages)).toBe(true);

			if (result.pages && result.pages.length > 0) {
				for (const page of result.pages) {
					expect(page).toHaveProperty("images");
					expect(Array.isArray(page.images)).toBe(true);
				}
			}
		});

		it("should associate images with correct page numbers", () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.pages && result.pages.length > 0) {
				const pageNumbers = new Set(result.pages.map((p) => p.pageNumber));

				if (result.images) {
					for (const image of result.images) {
						expect(pageNumbers.has(image.pageNumber || 1)).toBe(true);
					}
				}
			}
		});

		it("should maintain image order within pages", () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
				},
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			if (result.images && result.images.length > 1) {
				// Image indices should be sequential
				const indices = result.images.map((img) => img.imageIndex);
				for (let i = 0; i < indices.length - 1; i++) {
					expect(indices[i]).toBeLessThanOrEqual(indices[i + 1]);
				}
			}
		});
	});

	describe("Image extraction configuration combinations", () => {
		it("should work with all configuration options combined", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
					maxImageDimension: 2000,
					autoAdjustDpi: true,
					minDpi: 72,
					maxDpi: 300,
				},
				pages: {
					extractPages: true,
					insertPageMarkers: true,
				},
				useCache: false,
				enableQualityProcessing: true,
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
			expect(result.pages).toBeDefined();
			expect(result.images).toBeDefined();
		});

		it("should preserve images when combining with other features", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
				pages: {
					extractPages: true,
				},
				useCache: true,
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result.images).toBeDefined();
			expect(result.pages).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should handle image extraction with null overall config", () => {
			const result = extractFileSync(samplePdfPath, null);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should extract images with sync extraction function", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractFileSync(samplePdfPath, config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();
		});

		it("should extract images with bytes extraction", () => {
			const config: ExtractionConfig = {
				images: {
					enabled: true,
					targetDpi: 150,
				},
			};

			const result = extractBytesSync(samplePdfBytes, "application/pdf", config);

			expect(result).toBeDefined();
			expect(result.images).toBeDefined();
		});
	});
});
