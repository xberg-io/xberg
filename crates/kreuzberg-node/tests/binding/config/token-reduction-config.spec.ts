import { describe, expect, it } from "vitest";
import type { ExtractionConfig, TokenReductionConfig } from "../../src/types.js";

describe("TokenReductionConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: TokenReductionConfig = {};

			expect(config).toBeDefined();
			expect(config.mode).toBeUndefined();
		});

		it("should create config with conservative mode", () => {
			const config: TokenReductionConfig = {
				mode: "conservative",
			};

			expect(config.mode).toBe("conservative");
		});

		it("should create config with aggressive mode", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
			};

			expect(config.mode).toBe("aggressive");
		});

		it("should create config with preserveImportantWords", () => {
			const config: TokenReductionConfig = {
				preserveImportantWords: true,
			};

			expect(config.preserveImportantWords).toBe(true);
		});

		it("should create config with all fields", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
				preserveImportantWords: true,
			};

			expect(config.mode).toBe("aggressive");
			expect(config.preserveImportantWords).toBe(true);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: TokenReductionConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize mode to JSON", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("mode");
			expect(json).toContain("aggressive");
		});

		it("should serialize all fields to JSON", () => {
			const config: TokenReductionConfig = {
				mode: "conservative",
				preserveImportantWords: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("mode");
			expect(json).toContain("preserveImportantWords");
		});

		it("should deserialize from JSON", () => {
			const json = '{"mode":"aggressive","preserveImportantWords":false}';
			const config: TokenReductionConfig = JSON.parse(json);

			expect(config.mode).toBe("aggressive");
			expect(config.preserveImportantWords).toBe(false);
		});
	});

	describe("validation", () => {
		it("should accept conservative mode", () => {
			const config: TokenReductionConfig = { mode: "conservative" };
			expect(config.mode).toBe("conservative");
		});

		it("should accept aggressive mode", () => {
			const config: TokenReductionConfig = { mode: "aggressive" };
			expect(config.mode).toBe("aggressive");
		});

		it("should accept boolean values for preserveImportantWords", () => {
			const configTrue: TokenReductionConfig = { preserveImportantWords: true };
			const configFalse: TokenReductionConfig = { preserveImportantWords: false };

			expect(configTrue.preserveImportantWords).toBe(true);
			expect(configFalse.preserveImportantWords).toBe(false);
		});

		it("should type-check invalid mode at compile time", () => {
			// @ts-expect-error - invalid mode
			const _config: TokenReductionConfig = { mode: "invalid" };
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const tokenReductionConfig: TokenReductionConfig = {
				mode: "aggressive",
				preserveImportantWords: true,
			};

			const extractionConfig: ExtractionConfig = {
				tokenReduction: tokenReductionConfig,
			};

			expect(extractionConfig.tokenReduction?.mode).toBe("aggressive");
			expect(extractionConfig.tokenReduction?.preserveImportantWords).toBe(true);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				tokenReduction: {
					mode: "aggressive",
				},
				chunking: { maxChars: 2048 },
				ocr: { backend: "tesseract" },
			};

			expect(config.tokenReduction?.mode).toBe("aggressive");
			expect(config.chunking?.maxChars).toBe(2048);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined mode", () => {
			const config: TokenReductionConfig = {};

			expect(config.mode).toBeUndefined();
		});

		it("should handle undefined preserveImportantWords", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
			};

			expect(config.preserveImportantWords).toBeUndefined();
		});

		it("should handle null mode", () => {
			const config: TokenReductionConfig = {
				mode: null as any,
			};

			expect(config.mode).toBeNull();
		});

		it("should handle null preserveImportantWords", () => {
			const config: TokenReductionConfig = {
				preserveImportantWords: null as any,
			};

			expect(config.preserveImportantWords).toBeNull();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for mode", () => {
			const config: TokenReductionConfig = { mode: "aggressive" };
			expect(config).toHaveProperty("mode");
		});

		it("should use camelCase for preserveImportantWords", () => {
			const config: TokenReductionConfig = { preserveImportantWords: true };
			expect(config).toHaveProperty("preserveImportantWords");
		});

		it("should not have snake_case versions", () => {
			const config: TokenReductionConfig = {
				preserveImportantWords: true,
			};

			expect(config).not.toHaveProperty("preserve_important_words");
		});
	});

	describe("type safety", () => {
		it("should enforce string type for mode", () => {
			const config: TokenReductionConfig = { mode: "aggressive" };
			expect(typeof config.mode).toBe("string");
		});

		it("should enforce boolean type for preserveImportantWords", () => {
			const config: TokenReductionConfig = { preserveImportantWords: true };
			expect(typeof config.preserveImportantWords).toBe("boolean");
		});

		it("should reject non-boolean preserveImportantWords at compile time", () => {
			// @ts-expect-error - preserveImportantWords must be boolean
			const _config: TokenReductionConfig = { preserveImportantWords: "true" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: TokenReductionConfig = {
				mode: "aggressive",
				preserveImportantWords: true,
			};

			const updated: TokenReductionConfig = {
				...original,
				mode: "conservative",
			};

			expect(original.mode).toBe("aggressive");
			expect(updated.mode).toBe("conservative");
		});

		it("should support immutable updates", () => {
			const original: TokenReductionConfig = {
				mode: "aggressive",
			};

			const updated: TokenReductionConfig = {
				...original,
				preserveImportantWords: false,
			};

			expect(original.preserveImportantWords).toBeUndefined();
			expect(updated.preserveImportantWords).toBe(false);
		});
	});

	describe("reduction strategies", () => {
		it("should handle conservative reduction (safe)", () => {
			const config: TokenReductionConfig = {
				mode: "conservative",
				preserveImportantWords: true,
			};

			expect(config.mode).toBe("conservative");
			expect(config.preserveImportantWords).toBe(true);
		});

		it("should handle aggressive reduction (maximum)", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
				preserveImportantWords: false,
			};

			expect(config.mode).toBe("aggressive");
			expect(config.preserveImportantWords).toBe(false);
		});

		it("should handle aggressive with word preservation", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive",
				preserveImportantWords: true,
			};

			expect(config.mode).toBe("aggressive");
			expect(config.preserveImportantWords).toBe(true);
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: TokenReductionConfig = {
				preserveImportantWords: false,
			};

			expect(config.preserveImportantWords).toBe(false);
		});

		it("should handle empty mode string", () => {
			const config: TokenReductionConfig = {
				mode: "",
			};

			expect(config.mode).toBe("");
		});

		it("should handle mode with mixed case", () => {
			const config: TokenReductionConfig = {
				mode: "AgGrEsSiVe" as any,
			};

			expect(config.mode).toBe("AgGrEsSiVe");
		});

		it("should handle mode with special characters", () => {
			const config: TokenReductionConfig = {
				mode: "aggressive-v2" as any,
			};

			expect(config.mode).toBe("aggressive-v2");
		});

		it("should handle all fields with falsy values", () => {
			const config: TokenReductionConfig = {
				mode: "",
				preserveImportantWords: false,
			};

			expect(config.mode).toBe("");
			expect(config.preserveImportantWords).toBe(false);
		});
	});
});
