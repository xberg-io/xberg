import { describe, expect, it } from "vitest";
import type { ExtractionConfig, HierarchyConfig, PdfConfig } from "../../src/types.js";

describe("HierarchyConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: HierarchyConfig = {};

			expect(config).toBeDefined();
			expect(config.enabled).toBeUndefined();
		});

		it("should create config with enabled true", () => {
			const config: HierarchyConfig = {
				enabled: true,
			};

			expect(config.enabled).toBe(true);
		});

		it("should create config with enabled false", () => {
			const config: HierarchyConfig = {
				enabled: false,
			};

			expect(config.enabled).toBe(false);
		});

		it("should create config with kClusters", () => {
			const config: HierarchyConfig = {
				kClusters: 6,
			};

			expect(config.kClusters).toBe(6);
		});

		it("should create config with includeBbox", () => {
			const config: HierarchyConfig = {
				includeBbox: true,
			};

			expect(config.includeBbox).toBe(true);
		});

		it("should create config with ocrCoverageThreshold", () => {
			const config: HierarchyConfig = {
				ocrCoverageThreshold: 0.8,
			};

			expect(config.ocrCoverageThreshold).toBe(0.8);
		});

		it("should create config with all fields", () => {
			const config: HierarchyConfig = {
				enabled: true,
				kClusters: 8,
				includeBbox: true,
				ocrCoverageThreshold: 0.75,
			};

			expect(config.enabled).toBe(true);
			expect(config.kClusters).toBe(8);
			expect(config.includeBbox).toBe(true);
			expect(config.ocrCoverageThreshold).toBe(0.75);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: HierarchyConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize enabled to JSON", () => {
			const config: HierarchyConfig = {
				enabled: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
		});

		it("should serialize kClusters to JSON", () => {
			const config: HierarchyConfig = {
				kClusters: 6,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("kClusters");
			expect(json).toContain("6");
		});

		it("should serialize all fields to JSON", () => {
			const config: HierarchyConfig = {
				enabled: true,
				kClusters: 6,
				includeBbox: true,
				ocrCoverageThreshold: 0.8,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
			expect(json).toContain("kClusters");
			expect(json).toContain("includeBbox");
			expect(json).toContain("ocrCoverageThreshold");
		});

		it("should deserialize from JSON", () => {
			const json = '{"enabled":true,"kClusters":6}';
			const config: HierarchyConfig = JSON.parse(json);

			expect(config.enabled).toBe(true);
			expect(config.kClusters).toBe(6);
		});

		it("should deserialize with null threshold", () => {
			const json = '{"ocrCoverageThreshold":null}';
			const config: HierarchyConfig = JSON.parse(json);

			expect(config.ocrCoverageThreshold).toBeNull();
		});
	});

	describe("validation", () => {
		it("should accept boolean values for enabled", () => {
			const configTrue: HierarchyConfig = { enabled: true };
			const configFalse: HierarchyConfig = { enabled: false };

			expect(configTrue.enabled).toBe(true);
			expect(configFalse.enabled).toBe(false);
		});

		it("should accept valid kClusters values", () => {
			const values = [2, 3, 4, 5, 6, 8, 10];

			values.forEach((val) => {
				const config: HierarchyConfig = { kClusters: val };
				expect(config.kClusters).toBe(val);
			});
		});

		it("should accept boolean values for includeBbox", () => {
			const configTrue: HierarchyConfig = { includeBbox: true };
			const configFalse: HierarchyConfig = { includeBbox: false };

			expect(configTrue.includeBbox).toBe(true);
			expect(configFalse.includeBbox).toBe(false);
		});

		it("should accept valid threshold values", () => {
			const values = [0, 0.5, 0.8, 0.95, 1.0];

			values.forEach((val) => {
				const config: HierarchyConfig = { ocrCoverageThreshold: val };
				expect(config.ocrCoverageThreshold).toBe(val);
			});
		});

		it("should accept null threshold", () => {
			const config: HierarchyConfig = {
				ocrCoverageThreshold: null,
			};

			expect(config.ocrCoverageThreshold).toBeNull();
		});
	});

	describe("nesting", () => {
		it("should nest in PdfConfig", () => {
			const hierarchyConfig: HierarchyConfig = {
				enabled: true,
				kClusters: 6,
			};

			const pdfConfig: PdfConfig = {
				hierarchy: hierarchyConfig,
			};

			expect(pdfConfig.hierarchy?.enabled).toBe(true);
			expect(pdfConfig.hierarchy?.kClusters).toBe(6);
		});

		it("should nest in ExtractionConfig through PdfConfig", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					hierarchy: {
						enabled: true,
						kClusters: 8,
					},
				},
			};

			expect(config.pdfOptions?.hierarchy?.enabled).toBe(true);
			expect(config.pdfOptions?.hierarchy?.kClusters).toBe(8);
		});

		it("should nest with other PDF options", () => {
			const config: PdfConfig = {
				extractImages: true,
				hierarchy: {
					enabled: true,
					kClusters: 6,
					includeBbox: true,
				},
			};

			expect(config.extractImages).toBe(true);
			expect(config.hierarchy?.kClusters).toBe(6);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined enabled", () => {
			const config: HierarchyConfig = {};

			expect(config.enabled).toBeUndefined();
		});

		it("should handle undefined kClusters", () => {
			const config: HierarchyConfig = {
				enabled: true,
			};

			expect(config.kClusters).toBeUndefined();
		});

		it("should handle undefined includeBbox", () => {
			const config: HierarchyConfig = {
				enabled: true,
			};

			expect(config.includeBbox).toBeUndefined();
		});

		it("should handle undefined ocrCoverageThreshold", () => {
			const config: HierarchyConfig = {
				enabled: true,
			};

			expect(config.ocrCoverageThreshold).toBeUndefined();
		});

		it("should handle null ocrCoverageThreshold", () => {
			const config: HierarchyConfig = {
				ocrCoverageThreshold: null,
			};

			expect(config.ocrCoverageThreshold).toBeNull();
		});

		it("should allow mixing defined and undefined fields", () => {
			const config: HierarchyConfig = {
				enabled: true,
				ocrCoverageThreshold: 0.8,
			};

			expect(config.enabled).toBe(true);
			expect(config.kClusters).toBeUndefined();
			expect(config.ocrCoverageThreshold).toBe(0.8);
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for enabled", () => {
			const config: HierarchyConfig = { enabled: true };
			expect(config).toHaveProperty("enabled");
		});

		it("should use camelCase for kClusters", () => {
			const config: HierarchyConfig = { kClusters: 6 };
			expect(config).toHaveProperty("kClusters");
		});

		it("should use camelCase for includeBbox", () => {
			const config: HierarchyConfig = { includeBbox: true };
			expect(config).toHaveProperty("includeBbox");
		});

		it("should use camelCase for ocrCoverageThreshold", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 0.8 };
			expect(config).toHaveProperty("ocrCoverageThreshold");
		});

		it("should not have snake_case versions", () => {
			const config: HierarchyConfig = {
				includeBbox: true,
				ocrCoverageThreshold: 0.8,
			};

			expect(config).not.toHaveProperty("include_bbox");
			expect(config).not.toHaveProperty("ocr_coverage_threshold");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for enabled", () => {
			const config: HierarchyConfig = { enabled: true };
			expect(typeof config.enabled).toBe("boolean");
		});

		it("should enforce number type for kClusters", () => {
			const config: HierarchyConfig = { kClusters: 6 };
			expect(typeof config.kClusters).toBe("number");
		});

		it("should enforce boolean type for includeBbox", () => {
			const config: HierarchyConfig = { includeBbox: true };
			expect(typeof config.includeBbox).toBe("boolean");
		});

		it("should enforce number type for ocrCoverageThreshold", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 0.8 };
			expect(typeof config.ocrCoverageThreshold).toBe("number");
		});

		it("should reject non-boolean enabled at compile time", () => {
			// @ts-expect-error - enabled must be boolean
			const _config: HierarchyConfig = { enabled: 1 };
		});

		it("should reject non-number kClusters at compile time", () => {
			// @ts-expect-error - kClusters must be number
			const _config: HierarchyConfig = { kClusters: "6" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: HierarchyConfig = {
				enabled: true,
				kClusters: 6,
			};

			const updated: HierarchyConfig = {
				...original,
				kClusters: 8,
			};

			expect(original.kClusters).toBe(6);
			expect(updated.kClusters).toBe(8);
		});

		it("should support immutable updates with new fields", () => {
			const original: HierarchyConfig = {
				enabled: true,
			};

			const updated: HierarchyConfig = {
				...original,
				kClusters: 6,
				includeBbox: true,
			};

			expect(original.kClusters).toBeUndefined();
			expect(updated.kClusters).toBe(6);
		});
	});

	describe("kClusters configurations", () => {
		it("should handle minimum kClusters (2)", () => {
			const config: HierarchyConfig = { kClusters: 2 };
			expect(config.kClusters).toBe(2);
		});

		it("should handle typical kClusters (6)", () => {
			const config: HierarchyConfig = { kClusters: 6 };
			expect(config.kClusters).toBe(6);
		});

		it("should handle maximum kClusters (10)", () => {
			const config: HierarchyConfig = { kClusters: 10 };
			expect(config.kClusters).toBe(10);
		});

		it("should handle various power-of-2 values", () => {
			const values = [2, 4, 8, 16, 32];

			values.forEach((k) => {
				const config: HierarchyConfig = { kClusters: k };
				expect(config.kClusters).toBe(k);
			});
		});
	});

	describe("OCR coverage threshold configurations", () => {
		it("should handle 0 threshold (any OCR coverage)", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 0 };
			expect(config.ocrCoverageThreshold).toBe(0);
		});

		it("should handle 0.5 threshold (50% coverage)", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 0.5 };
			expect(config.ocrCoverageThreshold).toBe(0.5);
		});

		it("should handle 1.0 threshold (100% coverage)", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 1.0 };
			expect(config.ocrCoverageThreshold).toBe(1.0);
		});

		it("should handle null threshold (disabled)", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: null };
			expect(config.ocrCoverageThreshold).toBeNull();
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: HierarchyConfig = {
				enabled: false,
				includeBbox: false,
			};

			expect(config.enabled).toBe(false);
			expect(config.includeBbox).toBe(false);
		});

		it("should handle zero kClusters", () => {
			const config: HierarchyConfig = { kClusters: 0 };
			expect(config.kClusters).toBe(0);
		});

		it("should handle very large kClusters", () => {
			const config: HierarchyConfig = { kClusters: 1000 };
			expect(config.kClusters).toBe(1000);
		});

		it("should handle negative kClusters", () => {
			const config: HierarchyConfig = { kClusters: -6 };
			expect(config.kClusters).toBe(-6);
		});

		it("should handle threshold below 0", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: -0.5 };
			expect(config.ocrCoverageThreshold).toBe(-0.5);
		});

		it("should handle threshold above 1", () => {
			const config: HierarchyConfig = { ocrCoverageThreshold: 1.5 };
			expect(config.ocrCoverageThreshold).toBe(1.5);
		});

		it("should handle all fields with falsy values", () => {
			const config: HierarchyConfig = {
				enabled: false,
				kClusters: 0,
				includeBbox: false,
				ocrCoverageThreshold: 0,
			};

			expect(config.enabled).toBe(false);
			expect(config.kClusters).toBe(0);
			expect(config.includeBbox).toBe(false);
			expect(config.ocrCoverageThreshold).toBe(0);
		});

		it("should handle decimal kClusters values", () => {
			const config: HierarchyConfig = { kClusters: 6.5 };
			expect(config.kClusters).toBe(6.5);
		});

		it("should handle very precise threshold values", () => {
			const config: HierarchyConfig = {
				ocrCoverageThreshold: 0.123456789,
			};

			expect(config.ocrCoverageThreshold).toBe(0.123456789);
		});
	});
});
