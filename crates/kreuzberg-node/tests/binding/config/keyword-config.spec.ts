import { describe, expect, it } from "vitest";
import type { ExtractionConfig, KeywordConfig } from "../../src/types.js";

describe("KeywordConfig", () => {
	describe("construction", () => {
		it("should create config with default values", () => {
			const config: KeywordConfig = {};

			expect(config).toBeDefined();
			expect(config.algorithm).toBeUndefined();
		});

		it("should create config with yake algorithm", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
			};

			expect(config.algorithm).toBe("yake");
		});

		it("should create config with rake algorithm", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
			};

			expect(config.algorithm).toBe("rake");
		});

		it("should create config with maxKeywords", () => {
			const config: KeywordConfig = {
				maxKeywords: 20,
			};

			expect(config.maxKeywords).toBe(20);
		});

		it("should create config with minScore", () => {
			const config: KeywordConfig = {
				minScore: 0.5,
			};

			expect(config.minScore).toBe(0.5);
		});

		it("should create config with ngramRange", () => {
			const config: KeywordConfig = {
				ngramRange: [1, 3],
			};

			expect(config.ngramRange).toEqual([1, 3]);
		});

		it("should create config with language", () => {
			const config: KeywordConfig = {
				language: "en",
			};

			expect(config.language).toBe("en");
		});

		it("should create config with yakeParams", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
				yakeParams: {
					windowSize: 5,
				},
			};

			expect(config.yakeParams?.windowSize).toBe(5);
		});

		it("should create config with rakeParams", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
				rakeParams: {
					minWordLength: 4,
					maxWordsPerPhrase: 2,
				},
			};

			expect(config.rakeParams?.minWordLength).toBe(4);
			expect(config.rakeParams?.maxWordsPerPhrase).toBe(2);
		});

		it("should create config with all fields", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
				maxKeywords: 15,
				minScore: 0.2,
				ngramRange: [1, 2],
				language: "en",
				yakeParams: { windowSize: 4 },
			};

			expect(config.algorithm).toBe("yake");
			expect(config.maxKeywords).toBe(15);
			expect(config.minScore).toBe(0.2);
			expect(config.ngramRange).toEqual([1, 2]);
			expect(config.language).toBe("en");
		});
	});

	describe("serialization", () => {
		it("should serialize empty config to JSON", () => {
			const config: KeywordConfig = {};
			const json = JSON.stringify(config);

			expect(json).toBe("{}");
		});

		it("should serialize algorithm to JSON", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("algorithm");
		});

		it("should serialize all fields to JSON", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
				maxKeywords: 10,
				minScore: 0.1,
				language: "en",
			};

			const json = JSON.stringify(config);
			expect(json).toContain("algorithm");
			expect(json).toContain("maxKeywords");
			expect(json).toContain("minScore");
		});

		it("should deserialize from JSON", () => {
			const json = '{"algorithm":"yake","maxKeywords":10}';
			const config: KeywordConfig = JSON.parse(json);

			expect(config.algorithm).toBe("yake");
			expect(config.maxKeywords).toBe(10);
		});

		it("should deserialize with ngramRange", () => {
			const json = '{"ngramRange":[1,3],"language":"en"}';
			const config: KeywordConfig = JSON.parse(json);

			expect(config.ngramRange).toEqual([1, 3]);
			expect(config.language).toBe("en");
		});
	});

	describe("validation", () => {
		it("should accept yake algorithm", () => {
			const config: KeywordConfig = { algorithm: "yake" };
			expect(config.algorithm).toBe("yake");
		});

		it("should accept rake algorithm", () => {
			const config: KeywordConfig = { algorithm: "rake" };
			expect(config.algorithm).toBe("rake");
		});

		it("should type-check invalid algorithm at compile time", () => {
			// @ts-expect-error - invalid algorithm
			const _config: KeywordConfig = { algorithm: "invalid" };
		});

		it("should accept valid maxKeywords values", () => {
			const values = [1, 5, 10, 20, 100];

			values.forEach((val) => {
				const config: KeywordConfig = { maxKeywords: val };
				expect(config.maxKeywords).toBe(val);
			});
		});

		it("should accept valid minScore values", () => {
			const values = [0, 0.1, 0.5, 0.9, 1.0];

			values.forEach((val) => {
				const config: KeywordConfig = { minScore: val };
				expect(config.minScore).toBe(val);
			});
		});

		it("should accept various ngramRanges", () => {
			const ranges: Array<[number, number]> = [
				[1, 1],
				[1, 2],
				[1, 3],
				[2, 5],
			];

			ranges.forEach((range) => {
				const config: KeywordConfig = { ngramRange: range };
				expect(config.ngramRange).toEqual(range);
			});
		});
	});

	describe("nesting", () => {
		it("should nest in ExtractionConfig", () => {
			const keywordConfig: KeywordConfig = {
				algorithm: "yake",
				maxKeywords: 10,
			};

			const extractionConfig: ExtractionConfig = {
				keywords: keywordConfig,
			};

			expect(extractionConfig.keywords?.algorithm).toBe("yake");
			expect(extractionConfig.keywords?.maxKeywords).toBe(10);
		});

		it("should nest with other configs", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "rake",
					maxKeywords: 15,
				},
				chunking: { maxChars: 2048 },
				ocr: { backend: "tesseract" },
			};

			expect(config.keywords?.algorithm).toBe("rake");
			expect(config.chunking?.maxChars).toBe(2048);
		});
	});

	describe("optional fields", () => {
		it("should handle undefined algorithm", () => {
			const config: KeywordConfig = {};

			expect(config.algorithm).toBeUndefined();
		});

		it("should handle undefined maxKeywords", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
			};

			expect(config.maxKeywords).toBeUndefined();
		});

		it("should handle undefined minScore", () => {
			const config: KeywordConfig = {};

			expect(config.minScore).toBeUndefined();
		});

		it("should handle undefined ngramRange", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
			};

			expect(config.ngramRange).toBeUndefined();
		});

		it("should handle undefined language", () => {
			const config: KeywordConfig = {};

			expect(config.language).toBeUndefined();
		});

		it("should handle undefined yakeParams", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
			};

			expect(config.yakeParams).toBeUndefined();
		});

		it("should handle undefined rakeParams", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
			};

			expect(config.rakeParams).toBeUndefined();
		});
	});

	describe("camelCase properties", () => {
		it("should use camelCase for algorithm", () => {
			const config: KeywordConfig = { algorithm: "yake" };
			expect(config).toHaveProperty("algorithm");
		});

		it("should use camelCase for maxKeywords", () => {
			const config: KeywordConfig = { maxKeywords: 10 };
			expect(config).toHaveProperty("maxKeywords");
		});

		it("should use camelCase for minScore", () => {
			const config: KeywordConfig = { minScore: 0.5 };
			expect(config).toHaveProperty("minScore");
		});

		it("should use camelCase for ngramRange", () => {
			const config: KeywordConfig = { ngramRange: [1, 3] };
			expect(config).toHaveProperty("ngramRange");
		});

		it("should use camelCase for yakeParams and rakeParams", () => {
			const config: KeywordConfig = {
				yakeParams: { windowSize: 3 },
				rakeParams: { minWordLength: 3 },
			};

			expect(config).toHaveProperty("yakeParams");
			expect(config).toHaveProperty("rakeParams");
		});
	});

	describe("type safety", () => {
		it("should enforce string type for algorithm", () => {
			const config: KeywordConfig = { algorithm: "yake" };
			expect(typeof config.algorithm).toBe("string");
		});

		it("should enforce number type for maxKeywords", () => {
			const config: KeywordConfig = { maxKeywords: 10 };
			expect(typeof config.maxKeywords).toBe("number");
		});

		it("should enforce number type for minScore", () => {
			const config: KeywordConfig = { minScore: 0.5 };
			expect(typeof config.minScore).toBe("number");
		});

		it("should reject non-string algorithm at compile time", () => {
			// @ts-expect-error - algorithm must be string
			const _config: KeywordConfig = { algorithm: 123 };
		});

		it("should reject non-number maxKeywords at compile time", () => {
			// @ts-expect-error - maxKeywords must be number
			const _config: KeywordConfig = { maxKeywords: "10" };
		});
	});

	describe("immutability patterns", () => {
		it("should support spread operator for shallow copy", () => {
			const original: KeywordConfig = {
				algorithm: "yake",
				maxKeywords: 10,
			};

			const updated: KeywordConfig = {
				...original,
				maxKeywords: 20,
			};

			expect(original.maxKeywords).toBe(10);
			expect(updated.maxKeywords).toBe(20);
		});

		it("should support deep copy with params", () => {
			const original: KeywordConfig = {
				algorithm: "yake",
				yakeParams: { windowSize: 3 },
			};

			const updated: KeywordConfig = {
				...original,
				yakeParams: { ...original.yakeParams, windowSize: 5 },
			};

			expect(original.yakeParams?.windowSize).toBe(3);
			expect(updated.yakeParams?.windowSize).toBe(5);
		});
	});

	describe("algorithm-specific configurations", () => {
		it("should handle yake with windowSize", () => {
			const config: KeywordConfig = {
				algorithm: "yake",
				yakeParams: { windowSize: 5 },
			};

			expect(config.algorithm).toBe("yake");
			expect(config.yakeParams?.windowSize).toBe(5);
		});

		it("should handle rake with minWordLength", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
				rakeParams: { minWordLength: 4 },
			};

			expect(config.algorithm).toBe("rake");
			expect(config.rakeParams?.minWordLength).toBe(4);
		});

		it("should handle rake with maxWordsPerPhrase", () => {
			const config: KeywordConfig = {
				algorithm: "rake",
				rakeParams: { maxWordsPerPhrase: 3 },
			};

			expect(config.rakeParams?.maxWordsPerPhrase).toBe(3);
		});
	});

	describe("edge cases", () => {
		it("should handle zero maxKeywords", () => {
			const config: KeywordConfig = { maxKeywords: 0 };
			expect(config.maxKeywords).toBe(0);
		});

		it("should handle very large maxKeywords", () => {
			const config: KeywordConfig = { maxKeywords: 10000 };
			expect(config.maxKeywords).toBe(10000);
		});

		it("should handle minScore 0", () => {
			const config: KeywordConfig = { minScore: 0 };
			expect(config.minScore).toBe(0);
		});

		it("should handle minScore 1.0", () => {
			const config: KeywordConfig = { minScore: 1.0 };
			expect(config.minScore).toBe(1.0);
		});

		it("should handle ngram with single word", () => {
			const config: KeywordConfig = { ngramRange: [1, 1] };
			expect(config.ngramRange).toEqual([1, 1]);
		});

		it("should handle ngram with many words", () => {
			const config: KeywordConfig = { ngramRange: [1, 10] };
			expect(config.ngramRange).toEqual([1, 10]);
		});

		it("should handle empty language string", () => {
			const config: KeywordConfig = { language: "" };
			expect(config.language).toBe("");
		});

		it("should handle various language codes", () => {
			const langs = ["en", "de", "fr", "es", "zh", "ja", "pt"];

			langs.forEach((lang) => {
				const config: KeywordConfig = { language: lang };
				expect(config.language).toBe(lang);
			});
		});

		it("should handle zero windowSize", () => {
			const config: KeywordConfig = {
				yakeParams: { windowSize: 0 },
			};

			expect(config.yakeParams?.windowSize).toBe(0);
		});

		it("should handle zero minWordLength", () => {
			const config: KeywordConfig = {
				rakeParams: { minWordLength: 0 },
			};

			expect(config.rakeParams?.minWordLength).toBe(0);
		});
	});
});
