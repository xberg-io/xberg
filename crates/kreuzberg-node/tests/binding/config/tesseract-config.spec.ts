import { describe, expect, it } from "vitest";
import type { OcrConfig, TesseractConfig } from "../../src/types.js";

describe("TesseractConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: TesseractConfig = {};

			expect(config).toBeDefined();
			expect(config.psm).toBeUndefined();
			expect(config.enableTableDetection).toBeUndefined();
		});

		it("should create config with PSM only", () => {
			const config: TesseractConfig = {
				psm: 6,
			};

			expect(config.psm).toBe(6);
		});

		it("should create config with table detection", () => {
			const config: TesseractConfig = {
				enableTableDetection: true,
			};

			expect(config.enableTableDetection).toBe(true);
		});

		it("should create config with whitelist", () => {
			const config: TesseractConfig = {
				tesseditCharWhitelist: "0123456789",
			};

			expect(config.tesseditCharWhitelist).toBe("0123456789");
		});

		it("should create config with all fields", () => {
			const config: TesseractConfig = {
				psm: 3,
				enableTableDetection: true,
				tesseditCharWhitelist: "abcdefghijklmnopqrstuvwxyz",
			};

			expect(config.psm).toBe(3);
			expect(config.enableTableDetection).toBe(true);
			expect(config.tesseditCharWhitelist).toBe("abcdefghijklmnopqrstuvwxyz");
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: TesseractConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize with PSM to JSON", () => {
			const config: TesseractConfig = {
				psm: 6,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("psm");
			expect(json).toContain("6");
		});

		it("should serialize with all fields to JSON", () => {
			const config: TesseractConfig = {
				psm: 6,
				enableTableDetection: true,
				tesseditCharWhitelist: "0123456789",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("psm");
			expect(json).toContain("enableTableDetection");
			expect(json).toContain("tesseditCharWhitelist");
		});

		it("should deserialize from JSON", () => {
			const json = '{"psm":6,"enableTableDetection":true}';
			const config: TesseractConfig = JSON.parse(json);

			expect(config.psm).toBe(6);
			expect(config.enableTableDetection).toBe(true);
		});

		it("should deserialize with whitelist from JSON", () => {
			const json = '{"tesseditCharWhitelist":"0123456789"}';
			const config: TesseractConfig = JSON.parse(json);

			expect(config.tesseditCharWhitelist).toBe("0123456789");
		});
	});

	describe("validation", () => {
		it("should accept PSM 0 (OSD only)", () => {
			const config: TesseractConfig = { psm: 0 };
			expect(config.psm).toBe(0);
		});

		it("should accept PSM 3 (auto layout analysis)", () => {
			const config: TesseractConfig = { psm: 3 };
			expect(config.psm).toBe(3);
		});

		it("should accept PSM 6 (single uniform block)", () => {
			const config: TesseractConfig = { psm: 6 };
			expect(config.psm).toBe(6);
		});

		it("should accept PSM 11 (sparse text)", () => {
			const config: TesseractConfig = { psm: 11 };
			expect(config.psm).toBe(11);
		});

		it("should accept PSM 13 (raw line)", () => {
			const config: TesseractConfig = { psm: 13 };
			expect(config.psm).toBe(13);
		});

		it("should accept boolean values for enableTableDetection", () => {
			const configTrue: TesseractConfig = { enableTableDetection: true };
			const configFalse: TesseractConfig = { enableTableDetection: false };

			expect(configTrue.enableTableDetection).toBe(true);
			expect(configFalse.enableTableDetection).toBe(false);
		});

		it("should accept any string for whitelist", () => {
			const configs = ["0123456789", "abcdef", "ABCDEF", "!@#$%^&*()", ""];

			configs.forEach((whitelist) => {
				const config: TesseractConfig = { tesseditCharWhitelist: whitelist };
				expect(config.tesseditCharWhitelist).toBe(whitelist);
			});
		});
	});

	describe("nesting", () => {
		it("should nest in OcrConfig", () => {
			const tesseractConfig: TesseractConfig = {
				psm: 6,
				enableTableDetection: true,
			};

			const ocrConfig: OcrConfig = {
				backend: "tesseract",
				tesseractConfig,
			};

			expect(ocrConfig.tesseractConfig?.psm).toBe(6);
			expect(ocrConfig.tesseractConfig?.enableTableDetection).toBe(true);
		});

		it("should nest in OcrConfig with language", () => {
			const config: OcrConfig = {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 3,
					enableTableDetection: false,
				},
			};

			expect(config.language).toBe("eng");
			expect(config.tesseractConfig?.psm).toBe(3);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined psm", () => {
			const config: TesseractConfig = {};

			expect(config.psm).toBeUndefined();
		});

		it("should handle undefined enableTableDetection", () => {
			const config: TesseractConfig = {};

			expect(config.enableTableDetection).toBeUndefined();
		});

		it("should handle undefined tesseditCharWhitelist", () => {
			const config: TesseractConfig = {};

			expect(config.tesseditCharWhitelist).toBeUndefined();
		});

		it("should handle null enableTableDetection", () => {
			const config: TesseractConfig = {
				enableTableDetection: null as any,
			};

			expect(config.enableTableDetection).toBeNull();
		});

		it("should allow mixing defined and undefined fields", () => {
			const config: TesseractConfig = {
				psm: 6,
				tesseditCharWhitelist: "0123456789",
			};

			expect(config.psm).toBe(6);
			expect(config.enableTableDetection).toBeUndefined();
			expect(config.tesseditCharWhitelist).toBe("0123456789");
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for psm", () => {
			const config: TesseractConfig = { psm: 6 };
			expect(config).toHaveProperty("psm");
		});

		it("should use camelCase for enableTableDetection", () => {
			const config: TesseractConfig = { enableTableDetection: true };
			expect(config).toHaveProperty("enableTableDetection");
		});

		it("should use camelCase for tesseditCharWhitelist", () => {
			const config: TesseractConfig = { tesseditCharWhitelist: "abc" };
			expect(config).toHaveProperty("tesseditCharWhitelist");
		});

		it("should not have snake_case versions", () => {
			const config: TesseractConfig = {
				enableTableDetection: true,
				tesseditCharWhitelist: "abc",
			};

			expect(config).not.toHaveProperty("enable_table_detection");
			expect(config).not.toHaveProperty("tessedit_char_whitelist");
		});
	});

	describe("type safety", () => {
		it("should enforce number type for psm", () => {
			const config: TesseractConfig = { psm: 6 };
			expect(typeof config.psm).toBe("number");
		});

		it("should enforce boolean type for enableTableDetection", () => {
			const config: TesseractConfig = { enableTableDetection: true };
			expect(typeof config.enableTableDetection).toBe("boolean");
		});

		it("should enforce string type for whitelist", () => {
			const config: TesseractConfig = { tesseditCharWhitelist: "0123456789" };
			expect(typeof config.tesseditCharWhitelist).toBe("string");
		});

		it("should reject non-number psm at compile time", () => {
			// @ts-expect-error - psm must be number
			const _config: TesseractConfig = { psm: "6" };
		});

		it("should reject non-boolean enableTableDetection at compile time", () => {
			// @ts-expect-error - enableTableDetection must be boolean
			const _config: TesseractConfig = { enableTableDetection: "true" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: TesseractConfig = {
				psm: 6,
				enableTableDetection: true,
			};

			const updated: TesseractConfig = {
				...original,
				psm: 3,
			};

			expect(original.psm).toBe(6);
			expect(updated.psm).toBe(3);
		});

		it("should support immutable updates", () => {
			const original: TesseractConfig = {
				psm: 6,
				tesseditCharWhitelist: "0123456789",
			};

			const updated: TesseractConfig = {
				...original,
				enableTableDetection: true,
			};

			expect(original.enableTableDetection).toBeUndefined();
			expect(updated.enableTableDetection).toBe(true);
			expect(updated.psm).toBe(6);
		});
	});

	describe("common PSM values", () => {
		it("should handle common PSM configurations", () => {
			const psmConfigs = [
				{ psm: 0, description: "OSD only" },
				{ psm: 3, description: "Auto layout analysis" },
				{ psm: 6, description: "Single uniform block" },
				{ psm: 11, description: "Sparse text" },
				{ psm: 13, description: "Raw line" },
			];

			psmConfigs.forEach(({ psm }) => {
				const config: TesseractConfig = { psm };
				expect(config.psm).toBe(psm);
			});
		});
	});

	describe("edge cases", () => {
		it("should handle negative PSM values", () => {
			const config: TesseractConfig = { psm: -1 };
			expect(config.psm).toBe(-1);
		});

		it("should handle very large PSM values", () => {
			const config: TesseractConfig = { psm: 999 };
			expect(config.psm).toBe(999);
		});

		it("should handle very long whitelist", () => {
			const longWhitelist = "a".repeat(1000);
			const config: TesseractConfig = { tesseditCharWhitelist: longWhitelist };

			expect(config.tesseditCharWhitelist?.length).toBe(1000);
		});

		it("should handle whitelist with unicode characters", () => {
			const config: TesseractConfig = { tesseditCharWhitelist: "你好世界123" };
			expect(config.tesseditCharWhitelist).toBe("你好世界123");
		});

		it("should handle all falsy values", () => {
			const config: TesseractConfig = {
				psm: 0,
				enableTableDetection: false,
				tesseditCharWhitelist: "",
			};

			expect(config.psm).toBe(0);
			expect(config.enableTableDetection).toBe(false);
			expect(config.tesseditCharWhitelist).toBe("");
		});
	});
});
