import { describe, expect, it } from "vitest";
import type { ExtractionConfig, PageExtractionConfig } from "../../src/types.js";

describe("PageExtractionConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: PageExtractionConfig = {};

			expect(config).toBeDefined();
			expect(config.extractPages).toBeUndefined();
		});

		it("should create config with extractPages enabled", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
			};

			expect(config.extractPages).toBe(true);
		});

		it("should create config with extractPages disabled", () => {
			const config: PageExtractionConfig = {
				extractPages: false,
			};

			expect(config.extractPages).toBe(false);
		});

		it("should create config with insertPageMarkers", () => {
			const config: PageExtractionConfig = {
				insertPageMarkers: true,
			};

			expect(config.insertPageMarkers).toBe(true);
		});

		it("should create config with markerFormat", () => {
			const config: PageExtractionConfig = {
				markerFormat: "--- Page {page_num} ---",
			};

			expect(config.markerFormat).toBe("--- Page {page_num} ---");
		});

		it("should create config with all fields", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
				insertPageMarkers: true,
				markerFormat: "[Page {page_num}]",
			};

			expect(config.extractPages).toBe(true);
			expect(config.insertPageMarkers).toBe(true);
			expect(config.markerFormat).toBe("[Page {page_num}]");
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: PageExtractionConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize extractPages to JSON", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("extractPages");
		});

		it("should serialize all fields to JSON", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
				insertPageMarkers: true,
				markerFormat: "### Page {page_num}",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("extractPages");
			expect(json).toContain("insertPageMarkers");
			expect(json).toContain("markerFormat");
		});

		it("should deserialize from JSON", () => {
			const json = '{"extractPages":true,"insertPageMarkers":false}';
			const config: PageExtractionConfig = JSON.parse(json);

			expect(config.extractPages).toBe(true);
			expect(config.insertPageMarkers).toBe(false);
		});

		it("should deserialize with marker format", () => {
			const json = '{"markerFormat":"=== Page {page_num} ==="}';
			const config: PageExtractionConfig = JSON.parse(json);

			expect(config.markerFormat).toBe("=== Page {page_num} ===");
		});
	});

	describe("validation", () => {
		it("should accept boolean values for extractPages", () => {
			const configTrue: PageExtractionConfig = { extractPages: true };
			const configFalse: PageExtractionConfig = { extractPages: false };

			expect(configTrue.extractPages).toBe(true);
			expect(configFalse.extractPages).toBe(false);
		});

		it("should accept boolean values for insertPageMarkers", () => {
			const configTrue: PageExtractionConfig = { insertPageMarkers: true };
			const configFalse: PageExtractionConfig = { insertPageMarkers: false };

			expect(configTrue.insertPageMarkers).toBe(true);
			expect(configFalse.insertPageMarkers).toBe(false);
		});

		it("should accept various marker formats", () => {
			const formats = ["Page {page_num}", "--- Page {page_num} ---", "[{page_num}]", "Page: {page_num}", "p{page_num}"];

			formats.forEach((format) => {
				const config: PageExtractionConfig = { markerFormat: format };
				expect(config.markerFormat).toBe(format);
			});
		});

		it("should accept empty marker format", () => {
			const config: PageExtractionConfig = {
				markerFormat: "",
			};

			expect(config.markerFormat).toBe("");
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const pageConfig: PageExtractionConfig = {
				extractPages: true,
				insertPageMarkers: true,
			};

			const extractionConfig: ExtractionConfig = {
				pages: pageConfig,
			};

			expect(extractionConfig.pages?.extractPages).toBe(true);
			expect(extractionConfig.pages?.insertPageMarkers).toBe(true);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				pages: {
					extractPages: true,
					markerFormat: "[Page {page_num}]",
				},
				chunking: { maxChars: 2048 },
				keywords: { algorithm: "yake" },
			};

			expect(config.pages?.extractPages).toBe(true);
			expect(config.chunking?.maxChars).toBe(2048);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined extractPages", () => {
			const config: PageExtractionConfig = {};

			expect(config.extractPages).toBeUndefined();
		});

		it("should handle undefined insertPageMarkers", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
			};

			expect(config.insertPageMarkers).toBeUndefined();
		});

		it("should handle undefined markerFormat", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
			};

			expect(config.markerFormat).toBeUndefined();
		});

		it("should handle null markerFormat", () => {
			const config: PageExtractionConfig = {
				markerFormat: null as any,
			};

			expect(config.markerFormat).toBeNull();
		});

		it("should allow mixing defined and undefined fields", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
				markerFormat: "Page {page_num}",
			};

			expect(config.extractPages).toBe(true);
			expect(config.insertPageMarkers).toBeUndefined();
			expect(config.markerFormat).toBe("Page {page_num}");
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for extractPages", () => {
			const config: PageExtractionConfig = { extractPages: true };
			expect(config).toHaveProperty("extractPages");
		});

		it("should use camelCase for insertPageMarkers", () => {
			const config: PageExtractionConfig = { insertPageMarkers: true };
			expect(config).toHaveProperty("insertPageMarkers");
		});

		it("should use camelCase for markerFormat", () => {
			const config: PageExtractionConfig = { markerFormat: "test" };
			expect(config).toHaveProperty("markerFormat");
		});

		it("should not have snake_case versions", () => {
			const config: PageExtractionConfig = {
				extractPages: true,
				insertPageMarkers: true,
			};

			expect(config).not.toHaveProperty("extract_pages");
			expect(config).not.toHaveProperty("insert_page_markers");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for extractPages", () => {
			const config: PageExtractionConfig = { extractPages: true };
			expect(typeof config.extractPages).toBe("boolean");
		});

		it("should enforce boolean type for insertPageMarkers", () => {
			const config: PageExtractionConfig = { insertPageMarkers: true };
			expect(typeof config.insertPageMarkers).toBe("boolean");
		});

		it("should enforce string type for markerFormat", () => {
			const config: PageExtractionConfig = { markerFormat: "test" };
			expect(typeof config.markerFormat).toBe("string");
		});

		it("should reject non-boolean extractPages at compile time", () => {
			// @ts-expect-error - extractPages must be boolean
			const _config: PageExtractionConfig = { extractPages: "true" };
		});

		it("should reject non-string markerFormat at compile time", () => {
			// @ts-expect-error - markerFormat must be string
			const _config: PageExtractionConfig = { markerFormat: 123 };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: PageExtractionConfig = {
				extractPages: true,
				insertPageMarkers: false,
			};

			const updated: PageExtractionConfig = {
				...original,
				insertPageMarkers: true,
			};

			expect(original.insertPageMarkers).toBe(false);
			expect(updated.insertPageMarkers).toBe(true);
		});

		it("should support immutable updates with new fields", () => {
			const original: PageExtractionConfig = {
				extractPages: true,
			};

			const updated: PageExtractionConfig = {
				...original,
				markerFormat: "[{page_num}]",
			};

			expect(original.markerFormat).toBeUndefined();
			expect(updated.markerFormat).toBe("[{page_num}]");
		});
	});

	describe("marker format variations", () => {
		it("should handle simple page number format", () => {
			const config: PageExtractionConfig = {
				markerFormat: "Page {page_num}",
			};

			expect(config.markerFormat).toContain("page_num");
		});

		it("should handle markdown-style format", () => {
			const config: PageExtractionConfig = {
				markerFormat: "## Page {page_num}",
			};

			expect(config.markerFormat).toMatch(/^#/);
		});

		it("should handle bracket-style format", () => {
			const config: PageExtractionConfig = {
				markerFormat: "[Page {page_num}]",
			};

			expect(config.markerFormat).toMatch(/^\[.*\]$/);
		});

		it("should handle separator-style format", () => {
			const config: PageExtractionConfig = {
				markerFormat: "--- Page {page_num} ---",
			};

			expect(config.markerFormat).toMatch(/^-+/);
		});

		it("should handle format without page_num placeholder", () => {
			const config: PageExtractionConfig = {
				markerFormat: "---PAGE BREAK---",
			};

			expect(config.markerFormat).not.toContain("page_num");
		});

		it("should handle multiple page_num placeholders", () => {
			const config: PageExtractionConfig = {
				markerFormat: "Page {page_num} of {page_num}",
			};

			const count = (config.markerFormat?.match(/page_num/g) || []).length;
			expect(count).toBe(2);
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: PageExtractionConfig = {
				extractPages: false,
				insertPageMarkers: false,
			};

			expect(config.extractPages).toBe(false);
			expect(config.insertPageMarkers).toBe(false);
		});

		it("should handle very long marker format", () => {
			const longFormat = `Page {page_num} - ${"a".repeat(1000)}`;
			const config: PageExtractionConfig = {
				markerFormat: longFormat,
			};

			expect(config.markerFormat?.length).toBeGreaterThan(1000);
		});

		it("should handle marker format with special characters", () => {
			const config: PageExtractionConfig = {
				markerFormat: "🔖 Page {page_num} 🔖",
			};

			expect(config.markerFormat).toContain("🔖");
		});

		it("should handle marker format with newlines", () => {
			const config: PageExtractionConfig = {
				markerFormat: "Page {page_num}\n---\n",
			};

			expect(config.markerFormat).toContain("\n");
		});

		it("should handle marker format with regex-like characters", () => {
			const config: PageExtractionConfig = {
				markerFormat: "[Page {page_num}].*+",
			};

			expect(config.markerFormat).toMatch(/\[.*\]/);
		});

		it("should handle all fields with falsy values", () => {
			const config: PageExtractionConfig = {
				extractPages: false,
				insertPageMarkers: false,
				markerFormat: "",
			};

			expect(config.extractPages).toBe(false);
			expect(config.insertPageMarkers).toBe(false);
			expect(config.markerFormat).toBe("");
		});

		it("should handle marker format with various whitespace", () => {
			const config: PageExtractionConfig = {
				markerFormat: "  Page {page_num}  ",
			};

			expect(config.markerFormat).toMatch(/^\s+/);
		});
	});
});
