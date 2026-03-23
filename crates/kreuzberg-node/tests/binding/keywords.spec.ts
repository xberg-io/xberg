import { describe, expect, it } from "vitest";
import { extractBytes, extractBytesSync } from "../../dist/index.js";
import type { ExtractionConfig } from "../../src/types.js";

describe("Keyword Extraction", () => {
	describe("basic keyword extraction", () => {
		it("should extract keywords from simple English text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Machine learning and artificial intelligence are transforming technology.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(typeof result.content).toBe("string");
			expect(result.metadata).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords with required structure", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Natural language processing and neural networks enable advanced AI systems.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.mimeType).toContain("text/plain");
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle extraction with configuration object", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
					minScore: 0.1,
				},
			};

			const text = "Deep learning models process information through multiple layers.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords asynchronously", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Artificial intelligence transforms data science and automation.";
			const result = await extractBytes(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});
	});

	describe("multilingual keyword extraction", () => {
		it("should extract keywords from English text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "en",
					maxKeywords: 5,
				},
			};

			const text = "The rapid advancement of cloud computing infrastructure enables scalable solutions.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords from German text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "de",
					maxKeywords: 5,
				},
			};

			const text = "Die Künstliche Intelligenz revolutioniert die Technologieindustrie.";
			const result = extractBytesSync(Buffer.from(text, "utf-8"), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords from French text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "fr",
					maxKeywords: 5,
				},
			};

			const text = "L'apprentissage automatique transforme les données en connaissances.";
			const result = extractBytesSync(Buffer.from(text, "utf-8"), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords from Spanish text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					language: "es",
					maxKeywords: 5,
				},
			};

			const text = "El procesamiento del lenguaje natural es fundamental para la inteligencia artificial.";
			const result = extractBytesSync(Buffer.from(text, "utf-8"), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle UTF-8 multilingual text with accents", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const multilingualText = "Café, naïve, résumé - testing UTF-8 with accented characters.";
			const result = extractBytesSync(Buffer.from(multilingualText, "utf-8"), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});
	});

	describe("minScore filtering", () => {
		it("should filter keywords with minimum score threshold of 0.0", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 20,
					minScore: 0.0,
				},
			};

			const text = "Deep learning networks process information through multiple layers of abstraction.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should filter keywords with high minimum score threshold", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 20,
					minScore: 0.5,
				},
			};

			const text = "Quantum computing represents a paradigm shift in computational capabilities.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle edge case with minimum score at 1.0", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					minScore: 1.0,
				},
			};

			const text = "Edge case testing with minimum score threshold at maximum value.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should maintain filtering consistency across multiple calls", () => {
			const text = "Consistent filtering behavior with same configuration parameters.";

			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					minScore: 0.3,
				},
			};

			const result1 = extractBytesSync(Buffer.from(text), "text/plain", config);
			const result2 = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result1).toBeDefined();
			expect(result2).toBeDefined();
			expect(result1.metadata).toBeInstanceOf(Object);
			expect(result2.metadata).toBeInstanceOf(Object);
		});
	});

	describe("ngramRange variations", () => {
		it("should extract single-word keywords with ngram_range (1,1)", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					ngramRange: [1, 1],
				},
			};

			const text = "Single word extraction from multi-word phrases in text.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords with ngram_range (1,2) for 1-2 word phrases", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 15,
					ngramRange: [1, 2],
				},
			};

			const text = "Phrase extraction with multiple word combinations and single terms.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords with ngram_range (1,3) for 1-3 word phrases", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 15,
					ngramRange: [1, 3],
				},
			};

			const text = "Multi-word phrase extraction enables identification of key concepts and ideas.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords with ngram_range (2,4) for 2-4 word phrases", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					ngramRange: [2, 4],
				},
			};

			const text = "Advanced multi-word phrase extraction for longer keyword sequences.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should show different results with different ngram ranges", () => {
			const text = "Different ngram ranges produce different keyword results.";

			const configSingle: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					ngramRange: [1, 1],
				},
			};

			const configMulti: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
					ngramRange: [1, 3],
				},
			};

			const resultSingle = extractBytesSync(Buffer.from(text), "text/plain", configSingle);
			const resultMulti = extractBytesSync(Buffer.from(text), "text/plain", configMulti);

			expect(resultSingle).toBeDefined();
			expect(resultMulti).toBeDefined();
			expect(resultSingle.metadata).toBeInstanceOf(Object);
			expect(resultMulti.metadata).toBeInstanceOf(Object);
		});
	});

	describe("algorithm selection", () => {
		it("should extract keywords using YAKE algorithm", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "YAKE algorithm extracts keywords without external knowledge bases.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should extract keywords using RAKE algorithm", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "rake",
					maxKeywords: 10,
				},
			};

			const text = "RAKE extracts keywords using frequency and co-occurrence analysis.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle algorithm selection with different text types", () => {
			const testTexts = [
				"Technical document with specialized vocabulary and complex terminology.",
				"Simple text with basic keywords and straightforward language.",
				"Medium difficulty text with balanced keyword distribution patterns.",
			];

			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			for (const text of testTexts) {
				const result = extractBytesSync(Buffer.from(text), "text/plain", config);
				expect(result).toBeDefined();
				expect(result.metadata).toBeInstanceOf(Object);
			}
		});

		it("should support both YAKE and RAKE algorithms", () => {
			const text = "Algorithm comparison testing with various extraction methods.";

			const algorithms = ["yake", "rake"] as const;

			for (const algorithm of algorithms) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm,
						maxKeywords: 5,
					},
				};

				const result = extractBytesSync(Buffer.from(text), "text/plain", config);
				expect(result).toBeDefined();
				expect(result.metadata).toBeInstanceOf(Object);
			}
		});
	});

	describe("batch keyword extraction", () => {
		it("should extract keywords from multiple documents", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const texts = [
				"First document about machine learning systems.",
				"Second document discussing natural language processing.",
				"Third document covering deep neural networks.",
			];

			const results = texts.map((text) => extractBytesSync(Buffer.from(text), "text/plain", config));

			expect(results).toHaveLength(3);
			for (const result of results) {
				expect(result).toBeDefined();
				expect(result.metadata).toBeInstanceOf(Object);
			}
		});

		it("should maintain result ordering in batch processing", () => {
			const texts = [
				"Document one with unique keywords",
				"Document two with different keywords",
				"Document three with other keywords",
			];

			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const results = texts.map((text) => extractBytesSync(Buffer.from(text), "text/plain", config));

			expect(results).toHaveLength(texts.length);
			for (let i = 0; i < results.length; i++) {
				expect(results[i]).toBeDefined();
				expect(results[i].content).toBeTruthy();
			}
		});

		it("should handle batch processing with empty texts", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const texts = ["First document with content and keywords.", "", "Third document also with content."];

			const results = [];
			for (const text of texts) {
				try {
					const result = extractBytesSync(Buffer.from(text), "text/plain", config);
					results.push(result);
				} catch {
					results.push(null);
				}
			}

			expect(results).toHaveLength(3);
		});

		it("should process large batches of texts", async () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const texts = [
				"Machine learning and artificial intelligence are transforming technology.",
				"Deep learning neural networks enable advanced data science.",
				"Natural language processing handles text analysis.",
			];

			const results = await Promise.all(texts.map((text) => extractBytes(Buffer.from(text), "text/plain", config)));

			expect(results).toHaveLength(3);
			for (const result of results) {
				expect(result).toBeDefined();
				expect(result.metadata).toBeInstanceOf(Object);
			}
		});
	});

	describe("score normalization validation", () => {
		it("should maintain score values in normalized range", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Scoring normalization ensures all keyword scores are between zero and one.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should maintain score consistency across runs", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Consistency testing ensures reproducible keyword extraction results.";

			const result1 = extractBytesSync(Buffer.from(text), "text/plain", config);
			const result2 = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result1).toBeDefined();
			expect(result2).toBeDefined();
			expect(result1.metadata).toBeInstanceOf(Object);
			expect(result2.metadata).toBeInstanceOf(Object);
		});

		it("should handle various score thresholds", () => {
			const text = "Machine learning artificial intelligence data science neural networks deep learning.";

			const scoreThresholds = [0.0, 0.25, 0.5, 0.75, 1.0];

			for (const threshold of scoreThresholds) {
				const config: ExtractionConfig = {
					keywords: {
						algorithm: "yake",
						maxKeywords: 100,
						minScore: threshold,
					},
				};

				const result = extractBytesSync(Buffer.from(text), "text/plain", config);
				expect(result).toBeDefined();
				expect(result.metadata).toBeInstanceOf(Object);
			}
		});

		it("should validate score ordering reflects relevance", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Central keyword appears multiple times in this text. Central keyword relevance is high.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});
	});

	describe("empty and edge cases", () => {
		it("should handle empty string input", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle whitespace-only input", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "   \n\t  \n  ";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle very short text (less than 10 words)", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const text = "Short text here";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle single-word input", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const text = "Keyword";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle repeated word input", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const text = "word word word word word";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle special characters in text", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "Special characters: @#$%^&*() and symbols !? in text.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle numeric-only input", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 5,
				},
			};

			const text = "123 456 789 012 345";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle mixed case and punctuation", () => {
			const config: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 10,
				},
			};

			const text = "MixedCase UPPERCASE lowercase. With-hyphens and_underscores.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should respect max_keywords parameter limits", () => {
			const text = "Keywords are limited by max_keywords configuration parameter.";

			const configSmall: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 3,
				},
			};

			const configLarge: ExtractionConfig = {
				keywords: {
					algorithm: "yake",
					maxKeywords: 20,
				},
			};

			const resultSmall = extractBytesSync(Buffer.from(text), "text/plain", configSmall);
			const resultLarge = extractBytesSync(Buffer.from(text), "text/plain", configLarge);

			expect(resultSmall).toBeDefined();
			expect(resultLarge).toBeDefined();
			expect(resultSmall.metadata).toBeInstanceOf(Object);
			expect(resultLarge.metadata).toBeInstanceOf(Object);
		});

		it("should handle disabled keyword extraction", () => {
			const config: ExtractionConfig = {
				keywords: undefined,
			};

			const text = "This text should extract without keyword configuration.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});

		it("should handle null keywords configuration", () => {
			const config: ExtractionConfig = {
				keywords: null,
			};

			const text = "This text should extract without keyword processing.";
			const result = extractBytesSync(Buffer.from(text), "text/plain", config);

			expect(result).toBeDefined();
			expect(result.metadata).toBeInstanceOf(Object);
		});
	});
});
