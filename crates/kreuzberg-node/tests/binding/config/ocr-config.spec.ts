import { describe, expect, it } from "vitest";
import type { ExtractionConfig, OcrConfig } from "../../src/types.js";

describe("OcrConfig", () => {
	describe("construction", () => {
		it("should create config with default backend", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			expect(config.backend).toBe("tesseract");
		});

		it("should create config with custom backend and language", () => {
			const config: OcrConfig = {
				backend: "paddleocr",
				language: "fra",
			};

			expect(config.backend).toBe("paddleocr");
			expect(config.language).toBe("fra");
		});

		it("should create config with easyocr backend", () => {
			const config: OcrConfig = {
				backend: "easyocr",
				language: "deu",
			};

			expect(config.backend).toBe("easyocr");
			expect(config.language).toBe("deu");
		});

		it("should create config with tesseractConfig", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: {
					psm: 6,
					enableTableDetection: true,
				},
			};

			expect(config.tesseractConfig?.psm).toBe(6);
			expect(config.tesseractConfig?.enableTableDetection).toBe(true);
		});
	});

	describe("serialization", () => {
		it("should serialize to JSON with backend only", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("backend");
			expect(json).toContain("tesseract");
		});

		it("should serialize with all fields", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 3,
					enableTableDetection: false,
				},
			};

			const json = JSON.stringify(config);
			expect(json).toContain("tesseractConfig");
			expect(json).toContain("psm");
		});

		it("should deserialize from JSON", () => {
			const json = '{"backend":"tesseract","language":"eng"}';
			const config: OcrConfig = JSON.parse(json);

			expect(config.backend).toBe("tesseract");
			expect(config.language).toBe("eng");
		});

		it("should deserialize with nested config", () => {
			const json = '{"backend":"tesseract","tesseractConfig":{"psm":6}}';
			const config: OcrConfig = JSON.parse(json);

			expect(config.backend).toBe("tesseract");
			expect(config.tesseractConfig?.psm).toBe(6);
		});
	});

	describe("validation", () => {
		it("should accept tesseract backend", () => {
			const config: OcrConfig = { backend: "tesseract" };
			expect(config.backend).toBe("tesseract");
		});

		it("should accept paddleocr backend", () => {
			const config: OcrConfig = { backend: "paddleocr" };
			expect(config.backend).toBe("paddleocr");
		});

		it("should accept easyocr backend", () => {
			const config: OcrConfig = { backend: "easyocr" };
			expect(config.backend).toBe("easyocr");
		});

		it("should type-check invalid backend at compile time", () => {
			// @ts-expect-error - invalid backend should fail type checking
			const _config: OcrConfig = { backend: "invalid_backend" };
		});

		it("should accept valid language codes", () => {
			const languages = ["eng", "fra", "deu", "spa", "ita"];

			languages.forEach((lang) => {
				const config: OcrConfig = { backend: "tesseract", language: lang };
				expect(config.language).toBe(lang);
			});
		});

		it("should handle empty language string", () => {
			const config: OcrConfig = { backend: "tesseract", language: "" };
			expect(config.language).toBe("");
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const ocrConfig: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};

			const extractionConfig: ExtractionConfig = {
				ocr: ocrConfig,
			};

			expect(extractionConfig.ocr?.backend).toBe("tesseract");
			expect(extractionConfig.ocr?.language).toBe("eng");
		});

		it("should nest in ExtractionConfig with other settings", () => {
			const config: ExtractionConfig = {
				forceOcr: true,
				ocr: {
					backend: "paddleocr",
					language: "fra",
				},
				chunking: {
					maxChars: 2048,
				},
			};

			expect(config.forceOcr).toBe(true);
			expect(config.ocr?.backend).toBe("paddleocr");
			expect(config.chunking?.maxChars).toBe(2048);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			expect(config.language).toBeUndefined();
		});

		it("should handle undefined tesseractConfig", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			expect(config.tesseractConfig).toBeUndefined();
		});

		it("should handle null tesseractConfig", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: null as any,
			};

			expect(config.tesseractConfig).toBeNull();
		});

		it("should allow setting language without tesseractConfig", () => {
			const config: OcrConfig = {
				backend: "easyocr",
				language: "eng",
			};

			expect(config.language).toBe("eng");
			expect(config.tesseractConfig).toBeUndefined();
		});

		it("should allow tesseractConfig without language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { psm: 6 },
			};

			expect(config.tesseractConfig?.psm).toBe(6);
			expect(config.language).toBeUndefined();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for backend", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			expect(config).toHaveProperty("backend");
		});

		it("should use camelCase for language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};

			expect(config).toHaveProperty("language");
		});

		it("should use camelCase for tesseractConfig", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { psm: 3 },
			};

			expect(config).toHaveProperty("tesseractConfig");
		});
	});

	describe("type safety", () => {
		it("should enforce string type for backend", () => {
			const config: OcrConfig = {
				backend: "tesseract",
			};

			expect(typeof config.backend).toBe("string");
		});

		it("should enforce string type for language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};

			expect(typeof config.language).toBe("string");
		});

		it("should reject non-string backend at compile time", () => {
			// @ts-expect-error - backend must be string
			const _config: OcrConfig = { backend: 123 };
		});

		it("should reject non-string language at compile time", () => {
			// @ts-expect-error - language must be string
			const _config: OcrConfig = { backend: "tesseract", language: 123 };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: OcrConfig = {
				backend: "tesseract",
				language: "eng",
			};

			const updated: OcrConfig = {
				...original,
				language: "fra",
			};

			expect(original.language).toBe("eng");
			expect(updated.language).toBe("fra");
		});

		it("should support deep copy with tesseractConfig", () => {
			const original: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { psm: 6 },
			};

			const updated: OcrConfig = {
				...original,
				tesseractConfig: { ...original.tesseractConfig, enableTableDetection: true },
			};

			expect(original.tesseractConfig?.enableTableDetection).toBeUndefined();
			expect(updated.tesseractConfig?.enableTableDetection).toBe(true);
		});
	});

	describe("complex configurations", () => {
		it("should handle tesseract with all options", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 6,
					enableTableDetection: true,
					tesseditCharWhitelist: "0123456789",
				},
			};

			expect(config.backend).toBe("tesseract");
			expect(config.language).toBe("eng");
			expect(config.tesseractConfig?.psm).toBe(6);
			expect(config.tesseractConfig?.enableTableDetection).toBe(true);
			expect(config.tesseractConfig?.tesseditCharWhitelist).toBe("0123456789");
		});

		it("should handle multiple backend switches", () => {
			const backends: Array<OcrConfig["backend"]> = ["tesseract", "paddleocr", "easyocr"];

			backends.forEach((backend) => {
				const config: OcrConfig = { backend, language: "eng" };
				expect(config.backend).toBe(backend);
			});
		});
	});

	describe("edge cases", () => {
		it("should handle psm value 0", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { psm: 0 },
			};

			expect(config.tesseractConfig?.psm).toBe(0);
		});

		it("should handle psm value 13", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { psm: 13 },
			};

			expect(config.tesseractConfig?.psm).toBe(13);
		});

		it("should handle empty whitelist", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				tesseractConfig: { tesseditCharWhitelist: "" },
			};

			expect(config.tesseractConfig?.tesseditCharWhitelist).toBe("");
		});

		it("should handle very long language code", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng-usa-native",
			};

			expect(config.language).toBe("eng-usa-native");
		});

		it("should handle special characters in language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "zh-Hans",
			};

			expect(config.language).toBe("zh-Hans");
		});
	});
});
