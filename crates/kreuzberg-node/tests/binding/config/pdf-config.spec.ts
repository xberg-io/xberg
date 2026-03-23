import { describe, expect, it } from "vitest";
import type { ExtractionConfig, PdfConfig } from "../../src/types.js";

describe("PdfConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: PdfConfig = {};

			expect(config).toBeDefined();
			expect(config.extractImages).toBeUndefined();
		});

		it("should create config with extractImages enabled", () => {
			const config: PdfConfig = {
				extractImages: true,
			};

			expect(config.extractImages).toBe(true);
		});

		it("should create config with extractMetadata", () => {
			const config: PdfConfig = {
				extractMetadata: true,
			};

			expect(config.extractMetadata).toBe(true);
		});

		it("should create config with passwords", () => {
			const config: PdfConfig = {
				passwords: ["password123"],
			};

			expect(config.passwords).toEqual(["password123"]);
		});

		it("should create config with hierarchy", () => {
			const config: PdfConfig = {
				hierarchy: {
					enabled: true,
					kClusters: 6,
				},
			};

			expect(config.hierarchy?.enabled).toBe(true);
			expect(config.hierarchy?.kClusters).toBe(6);
		});

		it("should create config with all fields", () => {
			const config: PdfConfig = {
				extractImages: true,
				passwords: ["pass1", "pass2"],
				extractMetadata: true,
				hierarchy: {
					enabled: true,
					kClusters: 8,
					includeBbox: true,
				},
			};

			expect(config.extractImages).toBe(true);
			expect(config.passwords).toHaveLength(2);
			expect(config.extractMetadata).toBe(true);
			expect(config.hierarchy?.kClusters).toBe(8);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: PdfConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize with extractImages to JSON", () => {
			const config: PdfConfig = {
				extractImages: true,
				extractMetadata: false,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("extractImages");
			expect(json).toContain("extractMetadata");
		});

		it("should serialize with passwords array", () => {
			const config: PdfConfig = {
				passwords: ["secret", "password"],
			};

			const json = JSON.stringify(config);
			expect(json).toContain("passwords");
			expect(json).toContain("secret");
		});

		it("should serialize with hierarchy config", () => {
			const config: PdfConfig = {
				hierarchy: {
					enabled: true,
					kClusters: 6,
				},
			};

			const json = JSON.stringify(config);
			expect(json).toContain("hierarchy");
			expect(json).toContain("kClusters");
		});

		it("should deserialize from JSON", () => {
			const json = '{"extractImages":true,"extractMetadata":false}';
			const config: PdfConfig = JSON.parse(json);

			expect(config.extractImages).toBe(true);
			expect(config.extractMetadata).toBe(false);
		});

		it("should deserialize with passwords", () => {
			const json = '{"passwords":["pass1","pass2"]}';
			const config: PdfConfig = JSON.parse(json);

			expect(config.passwords).toEqual(["pass1", "pass2"]);
		});
	});

	describe("validation", () => {
		it("should accept boolean values for extractImages", () => {
			const configTrue: PdfConfig = { extractImages: true };
			const configFalse: PdfConfig = { extractImages: false };

			expect(configTrue.extractImages).toBe(true);
			expect(configFalse.extractImages).toBe(false);
		});

		it("should accept boolean values for extractMetadata", () => {
			const configTrue: PdfConfig = { extractMetadata: true };
			const configFalse: PdfConfig = { extractMetadata: false };

			expect(configTrue.extractMetadata).toBe(true);
			expect(configFalse.extractMetadata).toBe(false);
		});

		it("should accept single password", () => {
			const config: PdfConfig = {
				passwords: ["mypassword"],
			};

			expect(config.passwords).toHaveLength(1);
		});

		it("should accept multiple passwords", () => {
			const config: PdfConfig = {
				passwords: ["pass1", "pass2", "pass3"],
			};

			expect(config.passwords).toHaveLength(3);
		});

		it("should accept empty passwords array", () => {
			const config: PdfConfig = {
				passwords: [],
			};

			expect(config.passwords).toHaveLength(0);
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const pdfConfig: PdfConfig = {
				extractImages: true,
				extractMetadata: true,
			};

			const extractionConfig: ExtractionConfig = {
				pdfOptions: pdfConfig,
			};

			expect(extractionConfig.pdfOptions?.extractImages).toBe(true);
			expect(extractionConfig.pdfOptions?.extractMetadata).toBe(true);
		});

		it("should nest with other PDF-related configs", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					extractImages: true,
					extractMetadata: true,
				},
				images: {
					extractImages: true,
					targetDpi: 150,
				},
			};

			expect(config.pdfOptions?.extractImages).toBe(true);
			expect(config.images?.targetDpi).toBe(150);
		});

		it("should nest hierarchy config", () => {
			const config: PdfConfig = {
				hierarchy: {
					enabled: true,
					kClusters: 6,
				},
			};

			expect(config.hierarchy?.enabled).toBe(true);
			expect(config.hierarchy?.kClusters).toBe(6);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined extractImages", () => {
			const config: PdfConfig = {};

			expect(config.extractImages).toBeUndefined();
		});

		it("should handle undefined extractMetadata", () => {
			const config: PdfConfig = {
				extractImages: true,
			};

			expect(config.extractMetadata).toBeUndefined();
		});

		it("should handle undefined passwords", () => {
			const config: PdfConfig = {
				extractImages: true,
			};

			expect(config.passwords).toBeUndefined();
		});

		it("should handle undefined hierarchy", () => {
			const config: PdfConfig = {
				extractImages: true,
			};

			expect(config.hierarchy).toBeUndefined();
		});

		it("should handle null hierarchy", () => {
			const config: PdfConfig = {
				hierarchy: null as any,
			};

			expect(config.hierarchy).toBeNull();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for extractImages", () => {
			const config: PdfConfig = { extractImages: true };
			expect(config).toHaveProperty("extractImages");
		});

		it("should use camelCase for extractMetadata", () => {
			const config: PdfConfig = { extractMetadata: true };
			expect(config).toHaveProperty("extractMetadata");
		});

		it("should use camelCase for passwords", () => {
			const config: PdfConfig = { passwords: ["pass"] };
			expect(config).toHaveProperty("passwords");
		});

		it("should use camelCase for hierarchy", () => {
			const config: PdfConfig = { hierarchy: { enabled: true } };
			expect(config).toHaveProperty("hierarchy");
		});

		it("should not have snake_case versions", () => {
			const config: PdfConfig = {
				extractImages: true,
				extractMetadata: true,
			};

			expect(config).not.toHaveProperty("extract_images");
			expect(config).not.toHaveProperty("extract_metadata");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for extractImages", () => {
			const config: PdfConfig = { extractImages: true };
			expect(typeof config.extractImages).toBe("boolean");
		});

		it("should enforce boolean type for extractMetadata", () => {
			const config: PdfConfig = { extractMetadata: true };
			expect(typeof config.extractMetadata).toBe("boolean");
		});

		it("should enforce array type for passwords", () => {
			const config: PdfConfig = { passwords: ["pass"] };
			expect(Array.isArray(config.passwords)).toBe(true);
		});

		it("should reject non-boolean extractImages at compile time", () => {
			// @ts-expect-error - extractImages must be boolean
			const _config: PdfConfig = { extractImages: "true" };
		});

		it("should reject non-array passwords at compile time", () => {
			// @ts-expect-error - passwords must be array
			const _config: PdfConfig = { passwords: "password" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: PdfConfig = {
				extractImages: true,
				extractMetadata: false,
			};

			const updated: PdfConfig = {
				...original,
				extractMetadata: true,
			};

			expect(original.extractMetadata).toBe(false);
			expect(updated.extractMetadata).toBe(true);
		});

		it("should support deep copy with hierarchy", () => {
			const original: PdfConfig = {
				hierarchy: { enabled: true, kClusters: 6 },
			};

			const updated: PdfConfig = {
				...original,
				hierarchy: { ...original.hierarchy, kClusters: 8 },
			};

			expect(original.hierarchy?.kClusters).toBe(6);
			expect(updated.hierarchy?.kClusters).toBe(8);
		});

		it("should support immutable password updates", () => {
			const original: PdfConfig = {
				passwords: ["pass1", "pass2"],
			};

			const updated: PdfConfig = {
				...original,
				passwords: [...original.passwords!, "pass3"],
			};

			expect(original.passwords).toHaveLength(2);
			expect(updated.passwords).toHaveLength(3);
		});
	});

	describe("password handling", () => {
		it("should handle single password", () => {
			const config: PdfConfig = {
				passwords: ["secret123"],
			};

			expect(config.passwords).toContain("secret123");
		});

		it("should handle multiple passwords for fallback", () => {
			const config: PdfConfig = {
				passwords: ["oldpassword", "newpassword"],
			};

			expect(config.passwords).toHaveLength(2);
			expect(config.passwords![0]).toBe("oldpassword");
			expect(config.passwords![1]).toBe("newpassword");
		});

		it("should handle empty password", () => {
			const config: PdfConfig = {
				passwords: [""],
			};

			expect(config.passwords).toEqual([""]);
		});

		it("should handle passwords with special characters", () => {
			const config: PdfConfig = {
				passwords: ["p@$$w0rd!", "密码123"],
			};

			expect(config.passwords).toEqual(["p@$$w0rd!", "密码123"]);
		});
	});

	describe("hierarchy configuration", () => {
		it("should handle enabled hierarchy", () => {
			const config: PdfConfig = {
				hierarchy: { enabled: true },
			};

			expect(config.hierarchy?.enabled).toBe(true);
		});

		it("should handle disabled hierarchy", () => {
			const config: PdfConfig = {
				hierarchy: { enabled: false },
			};

			expect(config.hierarchy?.enabled).toBe(false);
		});

		it("should handle various kClusters values", () => {
			const values = [2, 4, 6, 8, 10];

			values.forEach((k) => {
				const config: PdfConfig = {
					hierarchy: { kClusters: k },
				};

				expect(config.hierarchy?.kClusters).toBe(k);
			});
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: PdfConfig = {
				extractImages: false,
				extractMetadata: false,
			};

			expect(config.extractImages).toBe(false);
			expect(config.extractMetadata).toBe(false);
		});

		it("should handle empty passwords array", () => {
			const config: PdfConfig = {
				passwords: [],
			};

			expect(config.passwords).toHaveLength(0);
		});

		it("should handle very long password array", () => {
			const passwords = Array.from({ length: 100 }, (_, i) => `pass${i}`);
			const config: PdfConfig = { passwords };

			expect(config.passwords).toHaveLength(100);
		});

		it("should handle very long password strings", () => {
			const longPassword = "a".repeat(10000);
			const config: PdfConfig = {
				passwords: [longPassword],
			};

			expect(config.passwords![0].length).toBe(10000);
		});

		it("should handle zero kClusters", () => {
			const config: PdfConfig = {
				hierarchy: { kClusters: 0 },
			};

			expect(config.hierarchy?.kClusters).toBe(0);
		});

		it("should handle very large kClusters", () => {
			const config: PdfConfig = {
				hierarchy: { kClusters: 1000 },
			};

			expect(config.hierarchy?.kClusters).toBe(1000);
		});
	});
});
