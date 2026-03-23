import { describe, expect, it } from "vitest";
import type { ExtractionConfig, PostProcessorConfig } from "../../src/types.js";

describe("PostProcessorConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: PostProcessorConfig = {};

			expect(config).toBeDefined();
			expect(config.enabled).toBeUndefined();
		});

		it("should create config with enabled true", () => {
			const config: PostProcessorConfig = {
				enabled: true,
			};

			expect(config.enabled).toBe(true);
		});

		it("should create config with enabled false", () => {
			const config: PostProcessorConfig = {
				enabled: false,
			};

			expect(config.enabled).toBe(false);
		});

		it("should create config with enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["processor1", "processor2"],
			};

			expect(config.enabledProcessors).toEqual(["processor1", "processor2"]);
		});

		it("should create config with disabledProcessors", () => {
			const config: PostProcessorConfig = {
				disabledProcessors: ["processor3", "processor4"],
			};

			expect(config.disabledProcessors).toEqual(["processor3", "processor4"]);
		});

		it("should create config with all fields", () => {
			const config: PostProcessorConfig = {
				enabled: true,
				enabledProcessors: ["proc1", "proc2"],
				disabledProcessors: ["proc3"],
			};

			expect(config.enabled).toBe(true);
			expect(config.enabledProcessors).toEqual(["proc1", "proc2"]);
			expect(config.disabledProcessors).toEqual(["proc3"]);
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: PostProcessorConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize enabled to JSON", () => {
			const config: PostProcessorConfig = {
				enabled: true,
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
		});

		it("should serialize enabledProcessors to JSON", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc1", "proc2"],
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabledProcessors");
			expect(json).toContain("proc1");
		});

		it("should serialize all fields to JSON", () => {
			const config: PostProcessorConfig = {
				enabled: true,
				enabledProcessors: ["a", "b"],
				disabledProcessors: ["c"],
			};

			const json = JSON.stringify(config);
			expect(json).toContain("enabled");
			expect(json).toContain("enabledProcessors");
			expect(json).toContain("disabledProcessors");
		});

		it("should deserialize from JSON", () => {
			const json = '{"enabled":false,"enabledProcessors":["proc1"]}';
			const config: PostProcessorConfig = JSON.parse(json);

			expect(config.enabled).toBe(false);
			expect(config.enabledProcessors).toEqual(["proc1"]);
		});

		it("should deserialize with disabled processors", () => {
			const json = '{"disabledProcessors":["skip1","skip2"]}';
			const config: PostProcessorConfig = JSON.parse(json);

			expect(config.disabledProcessors).toEqual(["skip1", "skip2"]);
		});
	});

	describe("validation", () => {
		it("should accept boolean values for enabled", () => {
			const configTrue: PostProcessorConfig = { enabled: true };
			const configFalse: PostProcessorConfig = { enabled: false };

			expect(configTrue.enabled).toBe(true);
			expect(configFalse.enabled).toBe(false);
		});

		it("should accept single processor in enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["processor1"],
			};

			expect(config.enabledProcessors).toHaveLength(1);
		});

		it("should accept multiple processors in enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc1", "proc2", "proc3"],
			};

			expect(config.enabledProcessors).toHaveLength(3);
		});

		it("should accept empty enabledProcessors array", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: [],
			};

			expect(config.enabledProcessors).toHaveLength(0);
		});

		it("should accept empty disabledProcessors array", () => {
			const config: PostProcessorConfig = {
				disabledProcessors: [],
			};

			expect(config.disabledProcessors).toHaveLength(0);
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const postProcConfig: PostProcessorConfig = {
				enabled: true,
				enabledProcessors: ["processor1"],
			};

			const extractionConfig: ExtractionConfig = {
				postprocessor: postProcConfig,
			};

			expect(extractionConfig.postprocessor?.enabled).toBe(true);
			expect(extractionConfig.postprocessor?.enabledProcessors).toEqual(["processor1"]);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				postprocessor: {
					enabled: true,
					enabledProcessors: ["proc1"],
				},
				keywords: { algorithm: "yake" },
				ocr: { backend: "tesseract" },
			};

			expect(config.postprocessor?.enabled).toBe(true);
			expect(config.keywords?.algorithm).toBe("yake");
		});
	});

	describe("optional fields", () => {
		it("should handle undefined enabled", () => {
			const config: PostProcessorConfig = {};

			expect(config.enabled).toBeUndefined();
		});

		it("should handle undefined enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabled: true,
			};

			expect(config.enabledProcessors).toBeUndefined();
		});

		it("should handle undefined disabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabled: true,
			};

			expect(config.disabledProcessors).toBeUndefined();
		});

		it("should handle null enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: null as any,
			};

			expect(config.enabledProcessors).toBeNull();
		});

		it("should handle null disabledProcessors", () => {
			const config: PostProcessorConfig = {
				disabledProcessors: null as any,
			};

			expect(config.disabledProcessors).toBeNull();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for enabled", () => {
			const config: PostProcessorConfig = { enabled: true };
			expect(config).toHaveProperty("enabled");
		});

		it("should use camelCase for enabledProcessors", () => {
			const config: PostProcessorConfig = { enabledProcessors: ["proc"] };
			expect(config).toHaveProperty("enabledProcessors");
		});

		it("should use camelCase for disabledProcessors", () => {
			const config: PostProcessorConfig = { disabledProcessors: ["proc"] };
			expect(config).toHaveProperty("disabledProcessors");
		});

		it("should not have snake_case versions", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc"],
				disabledProcessors: ["proc"],
			};

			expect(config).not.toHaveProperty("enabled_processors");
			expect(config).not.toHaveProperty("disabled_processors");
		});
	});

	describe("type safety", () => {
		it("should enforce boolean type for enabled", () => {
			const config: PostProcessorConfig = { enabled: true };
			expect(typeof config.enabled).toBe("boolean");
		});

		it("should enforce array type for enabledProcessors", () => {
			const config: PostProcessorConfig = { enabledProcessors: ["proc1"] };
			expect(Array.isArray(config.enabledProcessors)).toBe(true);
		});

		it("should enforce array type for disabledProcessors", () => {
			const config: PostProcessorConfig = { disabledProcessors: ["proc1"] };
			expect(Array.isArray(config.disabledProcessors)).toBe(true);
		});

		it("should reject non-boolean enabled at compile time", () => {
			// @ts-expect-error - enabled must be boolean
			const _config: PostProcessorConfig = { enabled: 1 };
		});

		it("should reject non-array enabledProcessors at compile time", () => {
			// @ts-expect-error - enabledProcessors must be array
			const _config: PostProcessorConfig = { enabledProcessors: "proc1" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: PostProcessorConfig = {
				enabled: true,
				enabledProcessors: ["proc1"],
			};

			const updated: PostProcessorConfig = {
				...original,
				enabled: false,
			};

			expect(original.enabled).toBe(true);
			expect(updated.enabled).toBe(false);
		});

		it("should support deep copy with processor arrays", () => {
			const original: PostProcessorConfig = {
				enabledProcessors: ["proc1", "proc2"],
			};

			const updated: PostProcessorConfig = {
				...original,
				enabledProcessors: [...original.enabledProcessors!, "proc3"],
			};

			expect(original.enabledProcessors).toHaveLength(2);
			expect(updated.enabledProcessors).toHaveLength(3);
		});

		it("should support immutable processor list updates", () => {
			const original: PostProcessorConfig = {
				disabledProcessors: ["skip1"],
			};

			const updated: PostProcessorConfig = {
				...original,
				disabledProcessors: ["skip2"],
			};

			expect(original.disabledProcessors).toEqual(["skip1"]);
			expect(updated.disabledProcessors).toEqual(["skip2"]);
		});
	});

	describe("processor list management", () => {
		it("should handle allowlist pattern with enabledProcessors", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["cleanup", "normalize", "validate"],
			};

			expect(config.enabledProcessors).toContain("cleanup");
			expect(config.enabledProcessors).toContain("normalize");
			expect(config.enabledProcessors).toHaveLength(3);
		});

		it("should handle denylist pattern with disabledProcessors", () => {
			const config: PostProcessorConfig = {
				disabledProcessors: ["experimental", "beta"],
			};

			expect(config.disabledProcessors).toContain("experimental");
			expect(config.disabledProcessors).toHaveLength(2);
		});

		it("should handle both allow and deny lists", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc1", "proc2"],
				disabledProcessors: ["proc3", "proc4"],
			};

			expect(config.enabledProcessors).toEqual(["proc1", "proc2"]);
			expect(config.disabledProcessors).toEqual(["proc3", "proc4"]);
		});

		it("should handle processor names with special characters", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc-v1", "proc_v2", "proc.v3"],
			};

			expect(config.enabledProcessors).toHaveLength(3);
		});
	});

	describe("edge cases", () => {
		it("should handle all falsy boolean values", () => {
			const config: PostProcessorConfig = {
				enabled: false,
			};

			expect(config.enabled).toBe(false);
		});

		it("should handle empty processor arrays", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: [],
				disabledProcessors: [],
			};

			expect(config.enabledProcessors).toHaveLength(0);
			expect(config.disabledProcessors).toHaveLength(0);
		});

		it("should handle very long processor names", () => {
			const longName = `processor_${"a".repeat(1000)}`;
			const config: PostProcessorConfig = {
				enabledProcessors: [longName],
			};

			expect(config.enabledProcessors![0].length).toBeGreaterThan(1000);
		});

		it("should handle many processors", () => {
			const processors = Array.from({ length: 100 }, (_, i) => `proc_${i}`);
			const config: PostProcessorConfig = {
				enabledProcessors: processors,
			};

			expect(config.enabledProcessors).toHaveLength(100);
		});

		it("should handle empty processor name strings", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["", "proc1", ""],
			};

			expect(config.enabledProcessors).toHaveLength(3);
			expect(config.enabledProcessors![0]).toBe("");
		});

		it("should handle processors with unicode names", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["处理器1", "proccsor_日本語"],
			};

			expect(config.enabledProcessors).toHaveLength(2);
		});

		it("should handle overlapping processor lists", () => {
			const config: PostProcessorConfig = {
				enabledProcessors: ["proc1", "proc2", "proc3"],
				disabledProcessors: ["proc2", "proc4"],
			};

			// Both proc2 in enabled and disabled - implementation-specific behavior
			expect(config.enabledProcessors).toContain("proc2");
			expect(config.disabledProcessors).toContain("proc2");
		});
	});
});
