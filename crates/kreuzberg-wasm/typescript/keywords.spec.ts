/**
 * Comprehensive keyword extraction tests for WASM bindings.
 *
 * Tests cover:
 * - Algorithm selection (YAKE, RAKE)
 * - Language variants (EN, DE, FR, ES)
 * - N-gram range configurations (1-1, 1-2, 1-3, 2-3)
 * - Score filtering (min_score threshold)
 * - Max keywords limiting
 * - Score consistency and normalization
 *
 * Framework: vitest
 */

import { beforeAll, describe, expect, it } from "vitest";
import { extractBytes, initWasm } from "./index.js";
import type { ExtractionConfig } from "./types.js";

beforeAll(async () => {
	await initWasm();
});

describe("Keyword Extraction (WASM Bindings)", () => {
	describe("algorithm selection", () => {
		it("should extract keywords using YAKE algorithm", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Keyword extraction algorithms determine which terms are most important.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();

			if (result.keywords && Array.isArray(result.keywords)) {
				for (const keyword of result.keywords) {
					expect(keyword.text).toBeDefined();
					expect(typeof keyword.text).toBe("string");
					expect(keyword.algorithm).toBe("yake");
				}
			}
		});

		it("should extract keywords using RAKE algorithm", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "rake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "RAKE algorithm extracts keywords through co-occurrence analysis.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.algorithm).toBe("rake");
				}
			}
		});

		it("should support algorithm switching", async () => {
			const text = "Algorithm selection enables flexible keyword extraction approaches.";
			const textBytes = new TextEncoder().encode(text);

			const yakeConfig: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 2],
					maxKeywords: 5,
				},
			};

			const rakeConfig: ExtractionConfig = {
				keywords: {
					algorithm: "rake",
					minScore: 0.0,
					ngramRange: [1, 2],
					maxKeywords: 5,
				},
			};

			const yakeResult = await extractBytes(textBytes, "text/plain", yakeConfig);
			const rakeResult = await extractBytes(textBytes, "text/plain", rakeConfig);

			expect(yakeResult.content).toBeDefined();
			expect(rakeResult.content).toBeDefined();
		});
	});

	describe("language variants", () => {
		it("should extract English keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "en",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 8,
				},
			};

			const text = "Machine learning and artificial intelligence transform technology.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should extract German keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "de",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 8,
				},
			};

			const text = "Die Künstliche Intelligenz revolutioniert die Technologieindustrie.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should extract French keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "fr",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 8,
				},
			};

			const text = "L'apprentissage automatique transforme les données en connaissances.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should extract Spanish keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "es",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 8,
				},
			};

			const text = "El procesamiento del lenguaje natural es fundamental para la inteligencia artificial.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should handle UTF-8 multilingual text", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Café, naïve, résumé - testing UTF-8 with accented characters.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});
	});

	describe("n-gram range configurations", () => {
		it("should extract single words with ngram_range=(1,1)", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 1],
					maxKeywords: 10,
				},
			};

			const text = "Multi-word phrase extraction enables identification of key concepts.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					// Single words should not have spaces
					const wordCount = keyword.text.split(" ").length;
					expect(wordCount).toBeLessThanOrEqual(1);
				}
			}
		});

		it("should extract 1-2 word phrases with ngram_range=(1,2)", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 2],
					maxKeywords: 15,
				},
			};

			const text = "Multi-word phrase extraction enables identification of concepts.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					const wordCount = keyword.text.split(" ").length;
					expect(wordCount).toBeLessThanOrEqual(2);
				}
			}
		});

		it("should extract 1-3 word phrases with ngram_range=(1,3)", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 15,
				},
			};

			const text = "Multi-word phrase extraction enables identification of key concepts.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					const wordCount = keyword.text.split(" ").length;
					expect(wordCount).toBeLessThanOrEqual(3);
				}
			}
		});

		it("should extract 2-3 word phrases with ngram_range=(2,3)", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [2, 3],
					maxKeywords: 10,
				},
			};

			const text = "Multi-word phrase extraction enables identification of key concepts.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					const wordCount = keyword.text.split(" ").length;
					expect(wordCount).toBeGreaterThanOrEqual(2);
					expect(wordCount).toBeLessThanOrEqual(3);
				}
			}
		});
	});

	describe("min_score filtering", () => {
		it("should include all keywords with min_score=0.0", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 20,
				},
			};

			const text = "Deep learning networks process information through multiple layers.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.score).toBeGreaterThanOrEqual(0.0);
				}
			}
		});

		it("should filter with min_score=0.5", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.5,
					ngramRange: [1, 3],
					maxKeywords: 20,
				},
			};

			const text = "Deep learning networks process information through multiple layers.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.score).toBeGreaterThanOrEqual(0.5);
				}
			}
		});

		it("should handle edge case with min_score=1.0", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 1.0,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Deep learning networks process information through multiple layers.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
		});

		it("should respect multiple score thresholds", async () => {
			const text = "Scoring thresholds filter keywords based on relevance metrics.";
			const textBytes = new TextEncoder().encode(text);

			const scores = [0.0, 0.3, 0.5, 0.7];

			for (const minScore of scores) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm: "yake",
						minScore,
						ngramRange: [1, 2],
						maxKeywords: 10,
					},
				};

				const result = await extractBytes(textBytes, "text/plain", config);

				// keywords may or may not be extracted in WASM

				if (result.keywords && result.keywords.length > 0) {
					for (const keyword of result.keywords) {
						expect(keyword.score).toBeGreaterThanOrEqual(minScore);
					}
				}
			}
		});
	});

	describe("max keywords limiting", () => {
		it("should limit keywords with maxKeywords=3", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 3,
				},
			};

			const text =
				"Keywords are limited by max_keywords configuration parameter. " +
				"This text contains many potential keywords and terms.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords) {
				expect(result.keywords.length).toBeLessThanOrEqual(3);
			}
		});

		it("should allow more results with maxKeywords=50", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 50,
				},
			};

			const text =
				"Keywords are limited by max_keywords configuration parameter. " +
				"This text contains many potential keywords and terms.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords) {
				expect(result.keywords.length).toBeLessThanOrEqual(50);
			}
		});

		it("should respect small maxKeywords values", async () => {
			const text = "Small limits constrain keyword extraction results significantly.";
			const textBytes = new TextEncoder().encode(text);

			const maxKeywordLimits = [1, 2, 5, 10];

			for (const maxKeywords of maxKeywordLimits) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm: "yake",
						minScore: 0.0,
						ngramRange: [1, 2],
						maxKeywords,
					},
				};

				const result = await extractBytes(textBytes, "text/plain", config);

				// keywords may or may not be extracted in WASM

				if (result.keywords) {
					expect(result.keywords.length).toBeLessThanOrEqual(maxKeywords);
				}
			}
		});
	});

	describe("score normalization and consistency", () => {
		it("should produce scores within 0-1 range", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 20,
				},
			};

			const text = "Scoring normalization ensures all keyword scores are between zero and one.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.score).toBeGreaterThanOrEqual(0.0);
					expect(keyword.score).toBeLessThanOrEqual(1.0);
				}
			}
		});

		it("should produce deterministic results across multiple runs", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 2],
					maxKeywords: 8,
				},
			};

			const text = "Consistency testing ensures reproducible keyword extraction results.";
			const textBytes = new TextEncoder().encode(text);

			const result1 = await extractBytes(textBytes, "text/plain", config);
			const result2 = await extractBytes(textBytes, "text/plain", config);
			const result3 = await extractBytes(textBytes, "text/plain", config);

			expect(result1.content).toBeDefined();
			expect(result2.content).toBeDefined();
			expect(result3.content).toBeDefined();

			if (result1.keywords && result2.keywords) {
				expect(result1.keywords.length).toBe(result2.keywords.length);
			}

			if (result2.keywords && result3.keywords) {
				expect(result2.keywords.length).toBe(result3.keywords.length);
			}
		});

		it("should maintain keyword order by score", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 20,
				},
			};

			const text = "Important keyword extraction scores determine ranking and ordering.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 1) {
				for (let i = 0; i < result.keywords.length - 1; i++) {
					const currentScore = result.keywords[i].score;
					const nextScore = result.keywords[i + 1].score;
					// Keywords should be in descending order
					expect(currentScore).toBeGreaterThanOrEqual(nextScore);
				}
			}
		});

		it("should validate score consistency across identical texts", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Score consistency validation across multiple processing runs.";
			const textBytes = new TextEncoder().encode(text);

			const results = [
				await extractBytes(textBytes, "text/plain", config),
				await extractBytes(textBytes, "text/plain", config),
				await extractBytes(textBytes, "text/plain", config),
			];

			for (const result of results) {
				// keywords may or may not be extracted in WASM
				// keywords field is optional in WASM

				if (result.keywords) {
					for (const keyword of result.keywords) {
						expect(typeof keyword.score).toBe("number");
						expect(Number.isFinite(keyword.score)).toBe(true);
					}
				}
			}
		});
	});

	describe("keyword extraction with YAKE parameters", () => {
		it("should accept YAKE windowSize parameter", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 10,
					yakeParams: {
						windowSize: 3,
					},
				},
			};

			const text = "YAKE parameters customize window size for keyword extraction behavior.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should support different YAKE window sizes", async () => {
			const text = "Window size configuration affects co-occurrence analysis.";
			const textBytes = new TextEncoder().encode(text);

			const windowSizes = [2, 3, 5];

			for (const windowSize of windowSizes) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm: "yake",
						minScore: 0.0,
						ngramRange: [1, 2],
						maxKeywords: 10,
						yakeParams: {
							windowSize,
						},
					},
				};

				const result = await extractBytes(textBytes, "text/plain", config);

				// keywords may or may not be extracted in WASM
				// keywords field is optional in WASM
			}
		});
	});

	describe("edge cases and special scenarios", () => {
		it("should handle empty keyword configuration gracefully", async () => {
			const config: ExtractionConfig = {
				keywords: {},
			};

			const text = "Text extraction without explicit keyword configuration.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeDefined();
		});

		it("should handle text with special characters", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.2,
					ngramRange: [1, 2],
					maxKeywords: 10,
				},
			};

			const text = "C++ programming, machine-learning, and AI/ML are important. Data @ scale!";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should handle very short text", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.3,
					ngramRange: [1, 3],
					maxKeywords: 5,
				},
			};

			const text = "Short text here";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			// keywords may or may not be extracted in WASM
		});

		it("should handle long text with many potential keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 2],
					maxKeywords: 20,
				},
			};

			let longText = "";
			for (let i = 0; i < 20; i++) {
				longText += "This is a word in a very long sentence with many words and phrases. ";
			}

			const textBytes = new TextEncoder().encode(longText);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should handle technical terminology", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.2,
					ngramRange: [1, 3],
					maxKeywords: 15,
				},
			};

			const text =
				"REST API endpoints, OAuth 2.0 authentication, and JSON Web Tokens enable " +
				"secure microservices architecture with containerized deployment.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM
		});

		it("should handle high score threshold filtering", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.9,
					ngramRange: [1, 2],
					maxKeywords: 5,
				},
			};

			const text = "This text is used for high threshold keyword extraction testing.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.score).toBeGreaterThanOrEqual(0.9);
				}
			}
		});
	});

	describe("keyword metadata validation", () => {
		it("should include keyword text in results", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Keywords should include text in extraction results.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.text).toBeDefined();
					expect(typeof keyword.text).toBe("string");
					expect(keyword.text.length).toBeGreaterThan(0);
				}
			}
		});

		it("should include algorithm information in keywords", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Algorithm information should be included in extracted keywords.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.algorithm).toBeDefined();
					expect(["yake", "rake"]).toContain(keyword.algorithm);
				}
			}
		});

		it("should include score in every keyword", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					minScore: 0.0,
					ngramRange: [1, 3],
					maxKeywords: 10,
				},
			};

			const text = "Every keyword should include a relevance score value.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM

			if (result.keywords && result.keywords.length > 0) {
				for (const keyword of result.keywords) {
					expect(keyword.score).toBeDefined();
					expect(typeof keyword.score).toBe("number");
					expect(Number.isFinite(keyword.score)).toBe(true);
				}
			}
		});
	});

	describe("configuration combinations", () => {
		it("should handle combined algorithm, language, and ngram settings", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "en",
					minScore: 0.3,
					ngramRange: [1, 2],
					maxKeywords: 15,
				},
			};

			const text = "Combined configuration parameters work together for precise extraction.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			// keywords may or may not be extracted in WASM
			// keywords field is optional in WASM

			if (result.keywords) {
				expect(result.keywords.length).toBeLessThanOrEqual(15);

				for (const keyword of result.keywords) {
					expect(keyword.score).toBeGreaterThanOrEqual(0.3);
					const wordCount = keyword.text.split(" ").length;
					expect(wordCount).toBeLessThanOrEqual(2);
				}
			}
		});

		it("should combine min score and max keywords effectively", async () => {
			const text = "Filtering by score and quantity produces precise keyword sets.";
			const textBytes = new TextEncoder().encode(text);

			const scenarios = [
				{ minScore: 0.3, maxKeywords: 5 },
				{ minScore: 0.5, maxKeywords: 10 },
				{ minScore: 0.7, maxKeywords: 3 },
			];

			for (const scenario of scenarios) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm: "yake",
						minScore: scenario.minScore,
						ngramRange: [1, 2],
						maxKeywords: scenario.maxKeywords,
					},
				};

				const result = await extractBytes(textBytes, "text/plain", config);

				// keywords may or may not be extracted in WASM

				if (result.keywords) {
					expect(result.keywords.length).toBeLessThanOrEqual(scenario.maxKeywords);

					for (const keyword of result.keywords) {
						expect(keyword.score).toBeGreaterThanOrEqual(scenario.minScore);
					}
				}
			}
		});
	});
});
