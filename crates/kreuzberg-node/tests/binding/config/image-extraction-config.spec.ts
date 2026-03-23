import { describe, expect, it } from "vitest";
import type { ExtractionConfig, ImageExtractionConfig } from "../../src/types.js";

describe("ImageExtractionConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: ImageExtractionConfig = {};

			expect(config).toBeDefined();
			expect(config.extractImages).toBeUndefined();
		});

		it("should create config with extractImages enabled", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
			};

			expect(config.extractImages).toBe(true);
		});

		it("should create config with targetDpi", () => {
			const config: ImageExtractionConfig = {
				targetDpi: 150,
			};

			expect(config.targetDpi).toBe(150);
		});

		it("should create config with maxImageDimension", () => {
			const config: ImageExtractionConfig = {
				maxImageDimension: 2000,
			};

			expect(config.maxImageDimension).toBe(2000);
		});

		it("should create config with autoAdjustDpi", () => {
			const config: ImageExtractionConfig = {
				autoAdjustDpi: true,
			};

			expect(config.autoAdjustDpi).toBe(true);
		});

		it("should create config with minDpi and maxDpi", () => {
			const config: ImageExtractionConfig = {
				minDpi: 72,
				maxDpi: 300,
			};

			expect(config.minDpi).toBe(72);
			expect(config.maxDpi).toBe(300);
		});

		it("should create config with all fields", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
				targetDpi: 150,
				maxImageDimension: 2000,
				autoAdjustDpi: true,
				minDpi: 72,
				maxDpi: 300,
			};

			expect(config.extractImages).toBe(true);
			expect(config.targetDpi).toBe(150);
			expect(config.maxImageDimension).toBe(2000);
			expect(config.autoAdjustDpi).toBe(true);
			expect(config.minDpi).toBe(72);
			expect(config.maxDpi).toBe(300);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: ImageExtractionConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize with extractImages to JSON", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("extractImages");
		});

		it("should serialize all fields to JSON", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
				targetDpi: 150,
				maxImageDimension: 2000,
				minDpi: 72,
				maxDpi: 300,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("targetDpi");
			expect(json).toContain("maxImageDimension");
			expect(json).toContain("minDpi");
			expect(json).toContain("maxDpi");
		});

		it("should deserialize from JSON", () => {
			const json = '{"extractImages":true,"targetDpi":150}';
			const config: ImageExtractionConfig = JSON.parse(json);

			expect(config.extractImages).toBe(true);
			expect(config.targetDpi).toBe(150);
		});

		it("should deserialize all DPI values", () => {
			const json = '{"targetDpi":150,"minDpi":72,"maxDpi":300,"maxImageDimension":2000}';
			const config: ImageExtractionConfig = JSON.parse(json);

			expect(config.targetDpi).toBe(150);
			expect(config.minDpi).toBe(72);
			expect(config.maxDpi).toBe(300);
			expect(config.maxImageDimension).toBe(2000);
		});
	});

	describe("validation", () => {
		it("should accept common DPI values", () => {
			const dpiValues = [72, 96, 150, 200, 300];

			dpiValues.forEach((dpi) => {
				const config: ImageExtractionConfig = { targetDpi: dpi };
				expect(config.targetDpi).toBe(dpi);
			});
		});

		it("should accept common image dimensions", () => {
			const dimensions = [500, 1000, 1500, 2000, 4000];

			dimensions.forEach((dim) => {
				const config: ImageExtractionConfig = { maxImageDimension: dim };
				expect(config.maxImageDimension).toBe(dim);
			});
		});

		it("should accept boolean values for extractImages", () => {
			const configTrue: ImageExtractionConfig = { extractImages: true };
			const configFalse: ImageExtractionConfig = { extractImages: false };

			expect(configTrue.extractImages).toBe(true);
			expect(configFalse.extractImages).toBe(false);
		});

		it("should accept boolean values for autoAdjustDpi", () => {
			const configTrue: ImageExtractionConfig = { autoAdjustDpi: true };
			const configFalse: ImageExtractionConfig = { autoAdjustDpi: false };

			expect(configTrue.autoAdjustDpi).toBe(true);
			expect(configFalse.autoAdjustDpi).toBe(false);
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const imageConfig: ImageExtractionConfig = {
				extractImages: true,
				targetDpi: 150,
			};

			const extractionConfig: ExtractionConfig = {
				images: imageConfig,
			};

			expect(extractionConfig.images?.extractImages).toBe(true);
			expect(extractionConfig.images?.targetDpi).toBe(150);
		});

		it("should nest with other image-related configs", () => {
			const config: ExtractionConfig = {
				images: {
					extractImages: true,
					targetDpi: 150,
					maxImageDimension: 2000,
				},
				pdfOptions: {
					extractImages: true,
				},
			};

			expect(config.images?.targetDpi).toBe(150);
			expect(config.pdfOptions?.extractImages).toBe(true);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined extractImages", () => {
			const config: ImageExtractionConfig = {};

			expect(config.extractImages).toBeUndefined();
		});

		it("should handle undefined targetDpi", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
			};

			expect(config.targetDpi).toBeUndefined();
		});

		it("should handle undefined maxImageDimension", () => {
			const config: ImageExtractionConfig = {};

			expect(config.maxImageDimension).toBeUndefined();
		});

		it("should handle undefined autoAdjustDpi", () => {
			const config: ImageExtractionConfig = {};

			expect(config.autoAdjustDpi).toBeUndefined();
		});

		it("should handle undefined minDpi and maxDpi", () => {
			const config: ImageExtractionConfig = {
				targetDpi: 150,
			};

			expect(config.minDpi).toBeUndefined();
			expect(config.maxDpi).toBeUndefined();
		});

		it("should handle null values", () => {
			const config: ImageExtractionConfig = {
				extractImages: null as any,
				targetDpi: null as any,
			};

			expect(config.extractImages).toBeNull();
			expect(config.targetDpi).toBeNull();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for extractImages", () => {
			const config: ImageExtractionConfig = { extractImages: true };
			expect(config).toHaveProperty("extractImages");
		});

		it("should use camelCase for targetDpi", () => {
			const config: ImageExtractionConfig = { targetDpi: 150 };
			expect(config).toHaveProperty("targetDpi");
		});

		it("should use camelCase for maxImageDimension", () => {
			const config: ImageExtractionConfig = { maxImageDimension: 2000 };
			expect(config).toHaveProperty("maxImageDimension");
		});

		it("should use camelCase for autoAdjustDpi", () => {
			const config: ImageExtractionConfig = { autoAdjustDpi: true };
			expect(config).toHaveProperty("autoAdjustDpi");
		});

		it("should use camelCase for minDpi and maxDpi", () => {
			const config: ImageExtractionConfig = { minDpi: 72, maxDpi: 300 };
			expect(config).toHaveProperty("minDpi");
			expect(config).toHaveProperty("maxDpi");
		});

		it("should not have snake_case versions", () => {
			const config: ImageExtractionConfig = {
				extractImages: true,
				targetDpi: 150,
			};

			expect(config).not.toHaveProperty("extract_images");
			expect(config).not.toHaveProperty("target_dpi");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for extractImages", () => {
			const config: ImageExtractionConfig = { extractImages: true };
			expect(typeof config.extractImages).toBe("boolean");
		});

		it("should enforce number type for DPI values", () => {
			const config: ImageExtractionConfig = {
				targetDpi: 150,
				minDpi: 72,
				maxDpi: 300,
			};

			expect(typeof config.targetDpi).toBe("number");
			expect(typeof config.minDpi).toBe("number");
			expect(typeof config.maxDpi).toBe("number");
		});

		it("should enforce number type for maxImageDimension", () => {
			const config: ImageExtractionConfig = { maxImageDimension: 2000 };
			expect(typeof config.maxImageDimension).toBe("number");
		});

		it("should reject non-number DPI at compile time", () => {
			// @ts-expect-error - targetDpi must be number
			const _config: ImageExtractionConfig = { targetDpi: "150" };
		});

		it("should reject non-boolean extractImages at compile time", () => {
			// @ts-expect-error - extractImages must be boolean
			const _config: ImageExtractionConfig = { extractImages: "true" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: ImageExtractionConfig = {
				extractImages: true,
				targetDpi: 150,
			};

			const updated: ImageExtractionConfig = {
				...original,
				targetDpi: 200,
			};

			expect(original.targetDpi).toBe(150);
			expect(updated.targetDpi).toBe(200);
		});

		it("should support immutable updates with new fields", () => {
			const original: ImageExtractionConfig = {
				extractImages: true,
			};

			const updated: ImageExtractionConfig = {
				...original,
				targetDpi: 150,
				maxImageDimension: 2000,
			};

			expect(original.targetDpi).toBeUndefined();
			expect(updated.targetDpi).toBe(150);
		});
	});

	describe("DPI configurations", () => {
		it("should handle standard DPI values", () => {
			const dpiConfigs = [
				{ targetDpi: 72, description: "Screen resolution" },
				{ targetDpi: 96, description: "Windows standard" },
				{ targetDpi: 150, description: "Good balance" },
				{ targetDpi: 300, description: "Print quality" },
			];

			dpiConfigs.forEach(({ targetDpi }) => {
				const config: ImageExtractionConfig = { targetDpi };
				expect(config.targetDpi).toBe(targetDpi);
			});
		});

		it("should handle minDpi less than maxDpi", () => {
			const config: ImageExtractionConfig = {
				minDpi: 72,
				maxDpi: 300,
			};

			expect(config.minDpi).toBeLessThan(config.maxDpi!);
		});

		it("should handle equal minDpi and maxDpi", () => {
			const config: ImageExtractionConfig = {
				minDpi: 150,
				maxDpi: 150,
			};

			expect(config.minDpi).toBe(config.maxDpi);
		});
	});

	describe("edge cases", () => {
		it("should handle zero DPI values", () => {
			const config: ImageExtractionConfig = {
				targetDpi: 0,
				minDpi: 0,
				maxDpi: 0,
			};

			expect(config.targetDpi).toBe(0);
			expect(config.minDpi).toBe(0);
			expect(config.maxDpi).toBe(0);
		});

		it("should handle very large DPI values", () => {
			const config: ImageExtractionConfig = {
				targetDpi: 10000,
				minDpi: 5000,
				maxDpi: 20000,
			};

			expect(config.targetDpi).toBe(10000);
			expect(config.maxDpi).toBe(20000);
		});

		it("should handle zero image dimension", () => {
			const config: ImageExtractionConfig = {
				maxImageDimension: 0,
			};

			expect(config.maxImageDimension).toBe(0);
		});

		it("should handle very large image dimensions", () => {
			const config: ImageExtractionConfig = {
				maxImageDimension: 100000,
			};

			expect(config.maxImageDimension).toBe(100000);
		});

		it("should handle all falsy boolean values", () => {
			const config: ImageExtractionConfig = {
				extractImages: false,
				autoAdjustDpi: false,
			};

			expect(config.extractImages).toBe(false);
			expect(config.autoAdjustDpi).toBe(false);
		});

		it("should handle negative DPI values", () => {
			const config: ImageExtractionConfig = {
				targetDpi: -150,
			};

			expect(config.targetDpi).toBe(-150);
		});

		it("should handle minDpi greater than maxDpi", () => {
			const config: ImageExtractionConfig = {
				minDpi: 300,
				maxDpi: 72,
			};

			expect(config.minDpi).toBeGreaterThan(config.maxDpi!);
		});
	});
});
