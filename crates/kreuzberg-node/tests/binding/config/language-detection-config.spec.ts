import { describe, expect, it } from "vitest";
import type { ExtractionConfig, LanguageDetectionConfig } from "../../src/types.js";

describe("LanguageDetectionConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: LanguageDetectionConfig = {};

			expect(config).toBeDefined();
			expect(config.enabled).toBeUndefined();
		});

		it("should create config with enabled true", () => {
			const config: LanguageDetectionConfig = {
				enabled: true,
			};

			expect(config.enabled).toBe(true);
		});

		it("should create config with enabled false", () => {
			const config: LanguageDetectionConfig = {
				enabled: false,
			};

			expect(config.enabled).toBe(false);
		});

		it("should create config with minConfidence", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.7,
			};

			expect(config.minConfidence).toBe(0.7);
		});

		it("should create config with detectMultiple", () => {
			const config: LanguageDetectionConfig = {
				detectMultiple: true,
			};

			expect(config.detectMultiple).toBe(true);
		});

		it("should create config with all fields", () => {
			const config: LanguageDetectionConfig = {
				enabled: true,
				minConfidence: 0.6,
				detectMultiple: true,
			};

			expect(config.enabled).toBe(true);
			expect(config.minConfidence).toBe(0.6);
			expect(config.detectMultiple).toBe(true);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: LanguageDetectionConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize enabled to JSON", () => {
			const config: LanguageDetectionConfig = {
				enabled: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
		});

		it("should serialize all fields to JSON", () => {
			const config: LanguageDetectionConfig = {
				enabled: true,
				minConfidence: 0.8,
				detectMultiple: false,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
			expect(json).toContain("minConfidence");
			expect(json).toContain("detectMultiple");
		});

		it("should deserialize from JSON", () => {
			const json = '{"enabled":true,"minConfidence":0.5}';
			const config: LanguageDetectionConfig = JSON.parse(json);

			expect(config.enabled).toBe(true);
			expect(config.minConfidence).toBe(0.5);
		});

		it("should deserialize with detectMultiple", () => {
			const json = '{"detectMultiple":true}';
			const config: LanguageDetectionConfig = JSON.parse(json);

			expect(config.detectMultiple).toBe(true);
		});
	});

	describe("validation", () => {
		it("should accept boolean values for enabled", () => {
			const configTrue: LanguageDetectionConfig = { enabled: true };
			const configFalse: LanguageDetectionConfig = { enabled: false };

			expect(configTrue.enabled).toBe(true);
			expect(configFalse.enabled).toBe(false);
		});

		it("should accept valid minConfidence values", () => {
			const values = [0, 0.1, 0.5, 0.9, 1.0];

			values.forEach((val) => {
				const config: LanguageDetectionConfig = { minConfidence: val };
				expect(config.minConfidence).toBe(val);
			});
		});

		it("should accept boolean values for detectMultiple", () => {
			const configTrue: LanguageDetectionConfig = { detectMultiple: true };
			const configFalse: LanguageDetectionConfig = { detectMultiple: false };

			expect(configTrue.detectMultiple).toBe(true);
			expect(configFalse.detectMultiple).toBe(false);
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const langDetectConfig: LanguageDetectionConfig = {
				enabled: true,
				minConfidence: 0.6,
			};

			const extractionConfig: ExtractionConfig = {
				languageDetection: langDetectConfig,
			};

			expect(extractionConfig.languageDetection?.enabled).toBe(true);
			expect(extractionConfig.languageDetection?.minConfidence).toBe(0.6);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				languageDetection: {
					enabled: true,
					detectMultiple: true,
				},
				ocr: { backend: "tesseract" },
			};

			expect(config.languageDetection?.detectMultiple).toBe(true);
			expect(config.ocr?.backend).toBe("tesseract");
		});
	});

	describe("optional fields", () => {
		it("should handle undefined enabled", () => {
			const config: LanguageDetectionConfig = {};

			expect(config.enabled).toBeUndefined();
		});

		it("should handle undefined minConfidence", () => {
			const config: LanguageDetectionConfig = {
				enabled: true,
			};

			expect(config.minConfidence).toBeUndefined();
		});

		it("should handle undefined detectMultiple", () => {
			const config: LanguageDetectionConfig = {};

			expect(config.detectMultiple).toBeUndefined();
		});

		it("should handle null minConfidence", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: null as any,
			};

			expect(config.minConfidence).toBeNull();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for enabled", () => {
			const config: LanguageDetectionConfig = { enabled: true };
			expect(config).toHaveProperty("enabled");
		});

		it("should use camelCase for minConfidence", () => {
			const config: LanguageDetectionConfig = { minConfidence: 0.5 };
			expect(config).toHaveProperty("minConfidence");
		});

		it("should use camelCase for detectMultiple", () => {
			const config: LanguageDetectionConfig = { detectMultiple: true };
			expect(config).toHaveProperty("detectMultiple");
		});

		it("should not have snake_case versions", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.5,
				detectMultiple: true,
			};

			expect(config).not.toHaveProperty("min_confidence");
			expect(config).not.toHaveProperty("detect_multiple");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for enabled", () => {
			const config: LanguageDetectionConfig = { enabled: true };
			expect(typeof config.enabled).toBe("boolean");
		});

		it("should enforce number type for minConfidence", () => {
			const config: LanguageDetectionConfig = { minConfidence: 0.5 };
			expect(typeof config.minConfidence).toBe("number");
		});

		it("should enforce boolean type for detectMultiple", () => {
			const config: LanguageDetectionConfig = { detectMultiple: true };
			expect(typeof config.detectMultiple).toBe("boolean");
		});

		it("should reject non-number minConfidence at compile time", () => {
			// @ts-expect-error - minConfidence must be number
			const _config: LanguageDetectionConfig = { minConfidence: "0.5" };
		});

		it("should reject non-boolean enabled at compile time", () => {
			// @ts-expect-error - enabled must be boolean
			const _config: LanguageDetectionConfig = { enabled: 1 };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: LanguageDetectionConfig = {
				enabled: true,
				minConfidence: 0.5,
			};

			const updated: LanguageDetectionConfig = {
				...original,
				minConfidence: 0.8,
			};

			expect(original.minConfidence).toBe(0.5);
			expect(updated.minConfidence).toBe(0.8);
		});

		it("should support immutable updates", () => {
			const original: LanguageDetectionConfig = {
				enabled: true,
			};

			const updated: LanguageDetectionConfig = {
				...original,
				detectMultiple: true,
				minConfidence: 0.6,
			};

			expect(original.detectMultiple).toBeUndefined();
			expect(updated.detectMultiple).toBe(true);
		});
	});

	describe("confidence thresholds", () => {
		it("should handle minimum confidence 0 (any language)", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0,
			};

			expect(config.minConfidence).toBe(0);
		});

		it("should handle confidence 0.5 (balanced)", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.5,
			};

			expect(config.minConfidence).toBe(0.5);
		});

		it("should handle high confidence 0.95", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.95,
			};

			expect(config.minConfidence).toBe(0.95);
		});

		it("should handle maximum confidence 1.0", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 1.0,
			};

			expect(config.minConfidence).toBe(1.0);
		});
	});

	describe("multi-language detection", () => {
		it("should handle detectMultiple true for multiple languages", () => {
			const config: LanguageDetectionConfig = {
				detectMultiple: true,
			};

			expect(config.detectMultiple).toBe(true);
		});

		it("should handle detectMultiple false for single language", () => {
			const config: LanguageDetectionConfig = {
				detectMultiple: false,
			};

			expect(config.detectMultiple).toBe(false);
		});

		it("should allow both true and false", () => {
			const configTrue: LanguageDetectionConfig = { detectMultiple: true };
			const configFalse: LanguageDetectionConfig = { detectMultiple: false };

			expect(configTrue.detectMultiple).not.toBe(configFalse.detectMultiple);
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: LanguageDetectionConfig = {
				enabled: false,
				detectMultiple: false,
			};

			expect(config.enabled).toBe(false);
			expect(config.detectMultiple).toBe(false);
		});

		it("should handle minConfidence slightly above 0", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.0001,
			};

			expect(config.minConfidence).toBeGreaterThan(0);
		});

		it("should handle minConfidence slightly below 1", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 0.9999,
			};

			expect(config.minConfidence).toBeLessThan(1);
		});

		it("should handle negative confidence", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: -0.5,
			};

			expect(config.minConfidence).toBe(-0.5);
		});

		it("should handle confidence greater than 1", () => {
			const config: LanguageDetectionConfig = {
				minConfidence: 1.5,
			};

			expect(config.minConfidence).toBeGreaterThan(1);
		});

		it("should handle all fields with falsy values", () => {
			const config: LanguageDetectionConfig = {
				enabled: false,
				minConfidence: 0,
				detectMultiple: false,
			};

			expect(config.enabled).toBe(false);
			expect(config.minConfidence).toBe(0);
			expect(config.detectMultiple).toBe(false);
		});
	});
});
