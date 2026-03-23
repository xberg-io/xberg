import { describe, expect, it } from "vitest";
import type { ExtractionConfig } from "../../src/types.js";

describe("ExtractionConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: ExtractionConfig = {};

			expect(config).toBeDefined();
			expect(config.useCache).toBeUndefined();
			expect(config.enableQualityProcessing).toBeUndefined();
		});

		it("should create config with useCache enabled", () => {
			const config: ExtractionConfig = {
				useCache: true,
			};

			expect(config.useCache).toBe(true);
		});

		it("should create config with useCache disabled", () => {
			const config: ExtractionConfig = {
				useCache: false,
			};

			expect(config.useCache).toBe(false);
		});

		it("should create config with enableQualityProcessing", () => {
			const config: ExtractionConfig = {
				enableQualityProcessing: true,
			};

			expect(config.enableQualityProcessing).toBe(true);
		});

		it("should create config with nested OCR config", () => {
			const config: ExtractionConfig = {
				ocr: {
					backend: "tesseract",
					language: "eng",
				},
			};

			expect(config.ocr?.backend).toBe("tesseract");
			expect(config.ocr?.language).toBe("eng");
		});

		it("should create config with all sub-configs", () => {
			const config: ExtractionConfig = {
				useCache: true,
				ocr: { backend: "tesseract" },
				chunking: { maxChars: 1024 },
				images: { extractImages: true },
				pdfOptions: { extractImages: true },
				tokenReduction: { mode: "aggressive" },
				languageDetection: { enabled: true },
				postprocessor: { enabled: true },
				keywords: { algorithm: "yake" },
				pages: { extractPages: true },
			};

			expect(config.useCache).toBe(true);
			expect(config.ocr).toBeDefined();
			expect(config.chunking).toBeDefined();
			expect(config.images).toBeDefined();
			expect(config.pdfOptions).toBeDefined();
			expect(config.tokenReduction).toBeDefined();
			expect(config.languageDetection).toBeDefined();
			expect(config.postprocessor).toBeDefined();
			expect(config.keywords).toBeDefined();
			expect(config.pages).toBeDefined();
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: ExtractionConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize config with values to JSON", () => {
			const config: ExtractionConfig = {
				useCache: true,
				enableQualityProcessing: false,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("useCache");
			expect(json).toContain("true");
		});

		it("should deserialize from JSON", () => {
			const json = '{"useCache":true,"enableQualityProcessing":false}';
			const config: ExtractionConfig = JSON.parse(json);

			expect(config.useCache).toBe(true);
			expect(config.enableQualityProcessing).toBe(false);
		});

		it("should deserialize nested configs from JSON", () => {
			const json = '{"ocr":{"backend":"tesseract","language":"eng"}}';
			const config: ExtractionConfig = JSON.parse(json);

			expect(config.ocr?.backend).toBe("tesseract");
			expect(config.ocr?.language).toBe("eng");
		});
	});

	describe("optional fields", () => {
		it("should handle undefined useCache", () => {
			const config: ExtractionConfig = {};

			expect(config.useCache).toBeUndefined();
		});

		it("should handle undefined enableQualityProcessing", () => {
			const config: ExtractionConfig = {};

			expect(config.enableQualityProcessing).toBeUndefined();
		});

		it("should handle null OCR config", () => {
			const config: ExtractionConfig = {
				ocr: null as any,
			};

			expect(config.ocr).toBeNull();
		});

		it("should handle undefined forceOcr", () => {
			const config: ExtractionConfig = {};

			expect(config.forceOcr).toBeUndefined();
		});

		it("should handle maxConcurrentExtractions", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 8,
			};

			expect(config.maxConcurrentExtractions).toBe(8);
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for useCache", () => {
			const config: ExtractionConfig = {
				useCache: true,
			};

			expect(config).toHaveProperty("useCache");
			expect(Object.keys(config)).toContain("useCache");
		});

		it("should use camelCase for enableQualityProcessing", () => {
			const config: ExtractionConfig = {
				enableQualityProcessing: true,
			};

			expect(config).toHaveProperty("enableQualityProcessing");
		});

		it("should use camelCase for forceOcr", () => {
			const config: ExtractionConfig = {
				forceOcr: true,
			};

			expect(config).toHaveProperty("forceOcr");
		});

		it("should use camelCase for maxConcurrentExtractions", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 4,
			};

			expect(config).toHaveProperty("maxConcurrentExtractions");
		});

		it("should use camelCase for all nested configs", () => {
			const config: ExtractionConfig = {
				pdfOptions: { extractImages: true },
				tokenReduction: { mode: "conservative" },
				languageDetection: { enabled: true },
				postprocessor: { enabled: true },
			};

			expect(config).toHaveProperty("pdfOptions");
			expect(config).toHaveProperty("tokenReduction");
			expect(config).toHaveProperty("languageDetection");
			expect(config).toHaveProperty("postprocessor");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for useCache", () => {
			const config: ExtractionConfig = {
				useCache: true,
			};

			expect(typeof config.useCache).toBe("boolean");
		});

		it("should enforce boolean type for enableQualityProcessing", () => {
			const config: ExtractionConfig = {
				enableQualityProcessing: false,
			};

			expect(typeof config.enableQualityProcessing).toBe("boolean");
		});

		it("should enforce number type for maxConcurrentExtractions", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 4,
			};

			expect(typeof config.maxConcurrentExtractions).toBe("number");
		});

		it("should reject invalid values at compile time", () => {
			// @ts-expect-error - useCache should be boolean
			const _config: ExtractionConfig = { useCache: "true" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: ExtractionConfig = {
				useCache: true,
				enableQualityProcessing: false,
			};

			const updated: ExtractionConfig = {
				...original,
				enableQualityProcessing: true,
			};

			expect(original.enableQualityProcessing).toBe(false);
			expect(updated.enableQualityProcessing).toBe(true);
		});

		it("should support deep copy with nested configs", () => {
			const original: ExtractionConfig = {
				ocr: { backend: "tesseract" },
			};

			const updated: ExtractionConfig = {
				...original,
				ocr: { ...original.ocr, language: "eng" },
			};

			expect(original.ocr?.language).toBeUndefined();
			expect(updated.ocr?.language).toBe("eng");
		});
	});

	describe("complex configurations", () => {
		it("should handle multiple chunking and keyword configs together", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 2048,
					maxOverlap: 256,
				},
				keywords: {
					algorithm: "rake",
					maxKeywords: 20,
					minScore: 0.2,
				},
			};

			expect(config.chunking?.maxChars).toBe(2048);
			expect(config.keywords?.algorithm).toBe("rake");
		});

		it("should handle forceOcr with OCR config", () => {
			const config: ExtractionConfig = {
				forceOcr: true,
				ocr: {
					backend: "tesseract",
					language: "eng",
					tesseractConfig: {
						psm: 6,
					},
				},
			};

			expect(config.forceOcr).toBe(true);
			expect(config.ocr?.tesseractConfig?.psm).toBe(6);
		});

		it("should handle PDF with hierarchy extraction", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					extractImages: true,
					hierarchy: {
						enabled: true,
						kClusters: 6,
					},
				},
			};

			expect(config.pdfOptions?.hierarchy?.enabled).toBe(true);
			expect(config.pdfOptions?.hierarchy?.kClusters).toBe(6);
		});
	});

	describe("edge cases", () => {
		it("should handle zero maxConcurrentExtractions", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 0,
			};

			expect(config.maxConcurrentExtractions).toBe(0);
		});

		it("should handle very large maxConcurrentExtractions", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 1000,
			};

			expect(config.maxConcurrentExtractions).toBe(1000);
		});

		it("should handle all configs with falsy values", () => {
			const config: ExtractionConfig = {
				useCache: false,
				enableQualityProcessing: false,
				forceOcr: false,
				maxConcurrentExtractions: 0,
			};

			expect(config.useCache).toBe(false);
			expect(config.enableQualityProcessing).toBe(false);
			expect(config.forceOcr).toBe(false);
			expect(config.maxConcurrentExtractions).toBe(0);
		});

		it("should handle empty nested configs", () => {
			const config: ExtractionConfig = {
				ocr: {},
				chunking: {},
				images: {},
				pdfOptions: {},
				keywords: {},
			};

			expect(config.ocr).toEqual({});
			expect(config.chunking).toEqual({});
			expect(config.images).toEqual({});
		});
	});
});
