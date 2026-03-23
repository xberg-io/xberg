import { describe, expect, it } from "vitest";
import type { ChunkingConfig, ExtractionConfig } from "../../src/types.js";

describe("ChunkingConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: ChunkingConfig = {};

			expect(config).toBeDefined();
			expect(config.maxChars).toBeUndefined();
		});

		it("should create config with maxChars", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
			};

			expect(config.maxChars).toBe(4096);
		});

		it("should create config with maxOverlap", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};

			expect(config.maxChars).toBe(4096);
			expect(config.maxOverlap).toBe(512);
		});

		it("should create config with chunkSize alternative", () => {
			const config: ChunkingConfig = {
				chunkSize: 1024,
			};

			expect(config.chunkSize).toBe(1024);
		});

		it("should create config with preset", () => {
			const config: ChunkingConfig = {
				preset: "aggressive",
			};

			expect(config.preset).toBe("aggressive");
		});

		it("should create config with enabled flag", () => {
			const config: ChunkingConfig = {
				enabled: true,
				maxChars: 2048,
			};

			expect(config.enabled).toBe(true);
			expect(config.maxChars).toBe(2048);
		});

		it("should create config with all fields", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
				chunkSize: 2048,
				chunkOverlap: 256,
				preset: "default",
				embedding: { model: "text-embedding-3" },
				enabled: true,
			};

			expect(config.maxChars).toBe(4096);
			expect(config.maxOverlap).toBe(512);
			expect(config.chunkSize).toBe(2048);
			expect(config.chunkOverlap).toBe(256);
			expect(config.preset).toBe("default");
			expect(config.embedding).toBeDefined();
			expect(config.enabled).toBe(true);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: ChunkingConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize maxChars to JSON", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("maxChars");
			expect(json).toContain("4096");
		});

		it("should deserialize from JSON", () => {
			const json = '{"maxChars":4096,"maxOverlap":512}';
			const config: ChunkingConfig = JSON.parse(json);

			expect(config.maxChars).toBe(4096);
			expect(config.maxOverlap).toBe(512);
		});

		it("should deserialize with preset from JSON", () => {
			const json = '{"preset":"aggressive"}';
			const config: ChunkingConfig = JSON.parse(json);

			expect(config.preset).toBe("aggressive");
		});

		it("should serialize with embedding config", () => {
			const config: ChunkingConfig = {
				maxChars: 2048,
				embedding: { model: "test", dimensions: 768 },
			};

			const json = JSON.stringify(config);
			expect(json).toContain("embedding");
			expect(json).toContain("model");
		});
	});

	describe("validation", () => {
		it("should accept valid maxChars values", () => {
			const values = [1024, 2048, 4096, 8192];

			values.forEach((val) => {
				const config: ChunkingConfig = { maxChars: val };
				expect(config.maxChars).toBe(val);
			});
		});

		it("should accept valid maxOverlap values", () => {
			const values = [0, 128, 256, 512, 1024];

			values.forEach((val) => {
				const config: ChunkingConfig = { maxChars: 4096, maxOverlap: val };
				expect(config.maxOverlap).toBe(val);
			});
		});

		it("should accept preset values", () => {
			const presets = ["default", "aggressive", "minimal"];

			presets.forEach((preset) => {
				const config: ChunkingConfig = { preset };
				expect(config.preset).toBe(preset);
			});
		});

		it("should accept enabled as boolean", () => {
			const configTrue: ChunkingConfig = { enabled: true };
			const configFalse: ChunkingConfig = { enabled: false };

			expect(configTrue.enabled).toBe(true);
			expect(configFalse.enabled).toBe(false);
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const chunkingConfig: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};

			const extractionConfig: ExtractionConfig = {
				chunking: chunkingConfig,
			};

			expect(extractionConfig.chunking?.maxChars).toBe(4096);
			expect(extractionConfig.chunking?.maxOverlap).toBe(512);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				chunking: { maxChars: 2048 },
				ocr: { backend: "tesseract" },
				keywords: { algorithm: "yake" },
			};

			expect(config.chunking?.maxChars).toBe(2048);
			expect(config.ocr?.backend).toBe("tesseract");
			expect(config.keywords?.algorithm).toBe("yake");
		});
	});

	describe("optional fields", () => {
		it("should handle undefined maxChars", () => {
			const config: ChunkingConfig = {};

			expect(config.maxChars).toBeUndefined();
		});

		it("should handle undefined maxOverlap", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
			};

			expect(config.maxOverlap).toBeUndefined();
		});

		it("should handle undefined preset", () => {
			const config: ChunkingConfig = {};

			expect(config.preset).toBeUndefined();
		});

		it("should handle null embedding", () => {
			const config: ChunkingConfig = {
				embedding: null as any,
			};

			expect(config.embedding).toBeNull();
		});

		it("should handle undefined enabled", () => {
			const config: ChunkingConfig = {
				maxChars: 2048,
			};

			expect(config.enabled).toBeUndefined();
		});

		it("should allow chunkSize without maxChars", () => {
			const config: ChunkingConfig = {
				chunkSize: 1024,
			};

			expect(config.chunkSize).toBe(1024);
			expect(config.maxChars).toBeUndefined();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for maxChars", () => {
			const config: ChunkingConfig = { maxChars: 4096 };
			expect(config).toHaveProperty("maxChars");
		});

		it("should use camelCase for maxOverlap", () => {
			const config: ChunkingConfig = { maxOverlap: 512 };
			expect(config).toHaveProperty("maxOverlap");
		});

		it("should use camelCase for chunkSize", () => {
			const config: ChunkingConfig = { chunkSize: 1024 };
			expect(config).toHaveProperty("chunkSize");
		});

		it("should use camelCase for chunkOverlap", () => {
			const config: ChunkingConfig = { chunkOverlap: 256 };
			expect(config).toHaveProperty("chunkOverlap");
		});

		it("should not have snake_case versions", () => {
			const config: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};

			expect(config).not.toHaveProperty("max_chars");
			expect(config).not.toHaveProperty("max_overlap");
		});
	});

	describe("type safety", () => {
		it("should enforce number type for maxChars", () => {
			const config: ChunkingConfig = { maxChars: 4096 };
			expect(typeof config.maxChars).toBe("number");
		});

		it("should enforce number type for maxOverlap", () => {
			const config: ChunkingConfig = { maxOverlap: 512 };
			expect(typeof config.maxOverlap).toBe("number");
		});

		it("should enforce string type for preset", () => {
			const config: ChunkingConfig = { preset: "default" };
			expect(typeof config.preset).toBe("string");
		});

		it("should enforce boolean type for enabled", () => {
			const config: ChunkingConfig = { enabled: true };
			expect(typeof config.enabled).toBe("boolean");
		});

		it("should reject non-number maxChars at compile time", () => {
			// @ts-expect-error - maxChars must be number
			const _config: ChunkingConfig = { maxChars: "4096" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: ChunkingConfig = {
				maxChars: 4096,
				maxOverlap: 512,
			};

			const updated: ChunkingConfig = {
				...original,
				maxChars: 2048,
			};

			expect(original.maxChars).toBe(4096);
			expect(updated.maxChars).toBe(2048);
		});

		it("should support deep copy with embedding", () => {
			const original: ChunkingConfig = {
				maxChars: 4096,
				embedding: { model: "ada" },
			};

			const updated: ChunkingConfig = {
				...original,
				embedding: { ...original.embedding, dimensions: 1536 },
			};

			expect(original.embedding?.dimensions).toBeUndefined();
			expect(updated.embedding?.dimensions).toBe(1536);
		});
	});

	describe("edge cases", () => {
		it("should handle zero maxChars", () => {
			const config: ChunkingConfig = { maxChars: 0 };
			expect(config.maxChars).toBe(0);
		});

		it("should handle very large maxChars", () => {
			const config: ChunkingConfig = { maxChars: 1000000 };
			expect(config.maxChars).toBe(1000000);
		});

		it("should handle negative overlap", () => {
			const config: ChunkingConfig = { maxOverlap: -1 };
			expect(config.maxOverlap).toBe(-1);
		});

		it("should handle overlap larger than chunk", () => {
			const config: ChunkingConfig = {
				maxChars: 100,
				maxOverlap: 200,
			};

			expect(config.maxChars).toBe(100);
			expect(config.maxOverlap).toBe(200);
		});

		it("should handle empty preset string", () => {
			const config: ChunkingConfig = { preset: "" };
			expect(config.preset).toBe("");
		});

		it("should handle complex embedding object", () => {
			const config: ChunkingConfig = {
				embedding: {
					model: "text-embedding-3-small",
					dimensions: 1536,
					normalize: true,
					pooling: "mean",
				},
			};

			expect(config.embedding).toBeDefined();
			expect(Object.keys(config.embedding)).toHaveLength(4);
		});

		it("should handle all falsy numeric values", () => {
			const config: ChunkingConfig = {
				maxChars: 0,
				maxOverlap: 0,
				chunkSize: 0,
				chunkOverlap: 0,
			};

			expect(config.maxChars).toBe(0);
			expect(config.maxOverlap).toBe(0);
			expect(config.chunkSize).toBe(0);
			expect(config.chunkOverlap).toBe(0);
		});
	});
});
