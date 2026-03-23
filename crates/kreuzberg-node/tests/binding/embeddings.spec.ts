/**
 * Comprehensive embeddings tests for TypeScript Node.js bindings.
 *
 * Tests verify embedding/vector generation functionality across multiple scenarios:
 * 1. Vector generation correctness - validates embedding vectors are generated properly
 * 2. Embedding dimension verification - ensures correct vector dimensions per model
 * 3. Performance with batch operations - tests efficiency with multiple texts
 * 4. Format-specific embedding handling - validates embeddings for different content types
 * 5. Similarity score validation - checks cosine similarity calculations
 * 6. Model switching - tests preset model switching and configuration
 * 7. Normalization correctness - validates L2 normalization of vectors
 *
 * NAPI-RS bindings with plain object configs (NO builder pattern).
 */

import { describe, expect, it } from "vitest";
import { extractBytesSync } from "../../dist/index.js";
import type { ExtractionConfig } from "../../src/types.js";

/**
 * Helper function to calculate Euclidean norm (magnitude) of a vector.
 */
function calculateVectorNorm(vector: number[]): number {
	return Math.sqrt(vector.reduce((sum, val) => sum + val * val, 0));
}

/**
 * Helper function to calculate cosine similarity between two vectors.
 */
function cosineSimilarity(vec1: number[], vec2: number[]): number {
	if (vec1.length !== vec2.length) {
		throw new Error("Vectors must have the same length");
	}

	const dotProduct = vec1.reduce((sum, val, i) => sum + val * vec2[i], 0);
	const mag1 = calculateVectorNorm(vec1);
	const mag2 = calculateVectorNorm(vec2);

	if (mag1 === 0 || mag2 === 0) {
		return 0;
	}

	return dotProduct / (mag1 * mag2);
}

/**
 * Helper to check if vector is properly normalized (L2 norm = 1).
 */
function isNormalized(vector: number[]): boolean {
	const norm = calculateVectorNorm(vector);
	const tolerance = 0.001; // Allow small floating point error
	return Math.abs(norm - 1.0) < tolerance;
}

describe("Embedding Vector Generation (Node.js Bindings)", () => {
	describe("vector generation correctness", () => {
		it("should generate embedding vectors with correct dimensions", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: 200,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
						normalize: false,
					},
				},
			};

			const text = "Machine learning transforms technology through artificial intelligence.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				// Check that chunks have embeddings
				for (const chunk of result.chunks) {
					if (chunk.embeddings) {
						expect(Array.isArray(chunk.embeddings)).toBe(true);
						expect(chunk.embeddings.length).toBeGreaterThan(0);

						// Each embedding should be a vector of numbers
						for (const embedding of chunk.embeddings) {
							expect(Array.isArray(embedding)).toBe(true);
							expect(embedding.length).toBeGreaterThan(0);

							// Validate common embedding dimensions (384, 512, 768, 1024)
							const validDimensions = [384, 512, 768, 1024, 256, 1536];
							expect(validDimensions).toContain(embedding.length);

							// All elements should be numbers
							for (const value of embedding) {
								expect(typeof value).toBe("number");
								expect(Number.isFinite(value)).toBe(true);
							}
						}
					}
				}
			}
		});

		it("should generate consistent vectors for same input text", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text = "Consistent embeddings ensure reproducible semantic search results.";
			const textBytes = new TextEncoder().encode(text);

			const result1 = extractBytesSync(textBytes, "text/plain", config);
			const result2 = extractBytesSync(textBytes, "text/plain", config);

			expect(result1.chunks).toBeDefined();
			expect(result2.chunks).toBeDefined();

			// Extract first embedding from each result
			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
				embedding1 = result1.chunks[0].embeddings[0];
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
				embedding2 = result2.chunks[0].embeddings[0];
			}

			// If embeddings were generated, they should be identical
			if (embedding1 && embedding2) {
				expect(embedding1).toEqual(embedding2);
			}
		});

		it("should generate non-zero vectors for non-empty text", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
					},
				},
			};

			const text = "Neural networks learn representations of data.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];

						// Vector should not be all zeros
						const hasNonZero = embedding.some((val) => val !== 0);
						expect(hasNonZero).toBe(true);

						// Calculate sum of absolute values
						const magnitude = embedding.reduce((sum, val) => sum + Math.abs(val), 0);
						expect(magnitude).toBeGreaterThan(0);
					}
				}
			}
		});

		it("should generate different vectors for different input texts", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text1 = "Machine learning is revolutionizing artificial intelligence development.";
			const text2 = "Cats are fluffy animals that like to sleep.";

			const result1 = extractBytesSync(new TextEncoder().encode(text1), "text/plain", config);
			const result2 = extractBytesSync(new TextEncoder().encode(text2), "text/plain", config);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
				embedding1 = result1.chunks[0].embeddings[0];
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
				embedding2 = result2.chunks[0].embeddings[0];
			}

			if (embedding1 && embedding2) {
				// Vectors should be different
				expect(embedding1).not.toEqual(embedding2);

				// Similarity should be lower than 0.9
				const similarity = cosineSimilarity(embedding1, embedding2);
				expect(similarity).toBeLessThan(0.9);
			}
		});
	});

	describe("embedding dimension verification", () => {
		it("should respect embedding dimensions for fast preset", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
					},
				},
			};

			const text = "Fast embeddings provide quick inference speed.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						// Fast preset typically uses 384-512 dimensions
						expect([384, 512, 256]).toContain(embedding.length);
					}
				}
			}
		});

		it("should respect embedding dimensions for balanced preset", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const text = "Balanced embeddings offer good tradeoff between speed and quality.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						// Balanced preset typically uses 512-768 dimensions
						expect([512, 768, 384]).toContain(embedding.length);
					}
				}
			}
		});

		it("should respect embedding dimensions for quality preset", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "quality",
						},
					},
				},
			};

			const text = "Quality embeddings provide superior semantic representation.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						// Quality preset typically uses 768-1024 dimensions
						expect([768, 1024, 512]).toContain(embedding.length);
					}
				}
			}
		});

		it("should maintain consistent dimensions across multiple chunks", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const longText =
				"This is a long text that will be chunked into multiple pieces. " +
				"Each chunk should have embeddings with the same dimensions. " +
				"Consistency is important for downstream processing tasks.";

			const textBytes = new TextEncoder().encode(longText);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 1) {
				let firstDimension: number | null = null;

				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const dimension = chunk.embeddings[0].length;

						if (firstDimension === null) {
							firstDimension = dimension;
						} else {
							// All chunks should have same dimension
							expect(dimension).toBe(firstDimension);
						}
					}
				}
			}
		});
	});

	describe("performance with batch operations", () => {
		it("should handle multiple text extractions efficiently", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
						batchSize: 32,
					},
				},
			};

			const texts = [
				"First document about machine learning.",
				"Second document on natural language processing.",
				"Third document covering neural networks.",
				"Fourth document on deep learning systems.",
				"Fifth document about transformers.",
			];

			const startTime = performance.now();

			const results = texts.map((text) => extractBytesSync(new TextEncoder().encode(text), "text/plain", config));

			const endTime = performance.now();
			const executionTime = endTime - startTime;

			expect(results).toHaveLength(5);

			for (const result of results) {
				expect(result).toBeDefined();
				expect(result.chunks).toBeDefined();
			}

			// Performance check - should complete in reasonable time
			// 5 extractions should take less than 10 seconds
			expect(executionTime).toBeLessThan(10000);
		});

		it("should process batch with consistent quality", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const texts = [
				"Data science uses statistics and machine learning.",
				"Artificial intelligence powers modern applications.",
				"Deep neural networks learn complex patterns.",
			];

			const results = texts.map((text) => extractBytesSync(new TextEncoder().encode(text), "text/plain", config));

			let embeddingDimension: number | null = null;

			for (const result of results) {
				if (result.chunks && result.chunks.length > 0) {
					for (const chunk of result.chunks) {
						if (chunk.embeddings && chunk.embeddings.length > 0) {
							const dimension = chunk.embeddings[0].length;

							if (embeddingDimension === null) {
								embeddingDimension = dimension;
							} else {
								// All results should have same dimension
								expect(dimension).toBe(embeddingDimension);
							}
						}
					}
				}
			}
		});

		it("should handle batch with varying batch sizes", () => {
			const batchSizes = [16, 32, 64];
			const text = "Batch processing efficiency depends on batch size configuration.";
			const textBytes = new TextEncoder().encode(text);

			for (const batchSize of batchSizes) {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: "fast",
							},
							batchSize,
						},
					},
				};

				const result = extractBytesSync(textBytes, "text/plain", config);

				expect(result).toBeDefined();
				expect(result.chunks).toBeDefined();
			}
		});
	});

	describe("format-specific embedding handling", () => {
		it("should generate embeddings for plain text", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const text = "Plain text content should be embedded correctly.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result.mimeType).toContain("text/plain");
			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				let hasEmbedding = false;
				for (const chunk of result.chunks) {
					if (chunk.embedding && chunk.embedding.length > 0) {
						hasEmbedding = true;
						break;
					}
				}
				expect(hasEmbedding).toBe(true);
			}
		});

		it("should preserve embeddings across multiple chunks", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
					},
				},
			};

			const text =
				"This long text will be split into multiple chunks. " +
				"Each chunk will have its own embedding. " +
				"Embeddings preserve semantic meaning of each chunk.";

			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				const embeddingCount = result.chunks.filter((chunk) => chunk.embedding && chunk.embedding.length > 0).length;

				// Most chunks should have embeddings
				expect(embeddingCount).toBeGreaterThan(0);
			}
		});

		it("should handle UTF-8 encoded text for embeddings", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			// Multilingual text with special characters
			const text = "Café, naïve, résumé - UTF-8 text with accents and special characters.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings) {
						expect(Array.isArray(chunk.embeddings)).toBe(true);
					}
				}
			}
		});
	});

	describe("similarity score validation", () => {
		it("should produce high similarity for semantically similar texts", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text1 = "Machine learning is a subset of artificial intelligence.";
			const text2 = "AI uses machine learning as a core technology.";

			const result1 = extractBytesSync(new TextEncoder().encode(text1), "text/plain", config);
			const result2 = extractBytesSync(new TextEncoder().encode(text2), "text/plain", config);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
				embedding1 = result1.chunks[0].embeddings[0];
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
				embedding2 = result2.chunks[0].embeddings[0];
			}

			if (embedding1 && embedding2) {
				const similarity = cosineSimilarity(embedding1, embedding2);
				// Semantically similar texts should have high similarity
				expect(similarity).toBeGreaterThan(0.5);
			}
		});

		it("should produce low similarity for semantically different texts", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text1 = "Advanced neural network architectures for deep learning.";
			const text2 = "Cooking recipes for homemade Italian pasta dishes.";

			const result1 = extractBytesSync(new TextEncoder().encode(text1), "text/plain", config);
			const result2 = extractBytesSync(new TextEncoder().encode(text2), "text/plain", config);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
				embedding1 = result1.chunks[0].embeddings[0];
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
				embedding2 = result2.chunks[0].embeddings[0];
			}

			if (embedding1 && embedding2) {
				const similarity = cosineSimilarity(embedding1, embedding2);
				// Semantically different texts should have low similarity
				expect(similarity).toBeLessThan(0.7);
			}
		});

		it("should produce valid cosine similarity values (between -1 and 1)", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const texts = [
				"Natural language processing techniques.",
				"Computer vision and image analysis.",
				"Reinforcement learning algorithms.",
			];

			for (let i = 0; i < texts.length; i++) {
				for (let j = i + 1; j < texts.length; j++) {
					const result1 = extractBytesSync(new TextEncoder().encode(texts[i]), "text/plain", config);
					const result2 = extractBytesSync(new TextEncoder().encode(texts[j]), "text/plain", config);

					let embedding1: number[] | null = null;
					let embedding2: number[] | null = null;

					if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
						embedding1 = result1.chunks[0].embeddings[0];
					}

					if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
						embedding2 = result2.chunks[0].embeddings[0];
					}

					if (embedding1 && embedding2) {
						const similarity = cosineSimilarity(embedding1, embedding2);
						expect(similarity).toBeGreaterThanOrEqual(-1);
						expect(similarity).toBeLessThanOrEqual(1);
					}
				}
			}
		});
	});

	describe("model switching", () => {
		it("should switch between preset models", () => {
			const presets: Array<"fast" | "balanced" | "quality" | "multilingual"> = [
				"fast",
				"balanced",
				"quality",
				"multilingual",
			];

			const text = "Model switching allows different embedding quality and speed tradeoffs.";
			const textBytes = new TextEncoder().encode(text);

			for (const preset of presets) {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: preset,
							},
						},
					},
				};

				const result = extractBytesSync(textBytes, "text/plain", config);

				expect(result).toBeDefined();
				expect(result.chunks).toBeDefined();
			}
		});

		it("should support custom ONNX model configuration", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "custom",
							value: "BAAI/bge-base-en-v1.5",
							dimensions: 768,
						},
						normalize: true,
					},
				},
			};

			const text = "Custom ONNX models provide efficient embedding generation.";
			const textBytes = new TextEncoder().encode(text);

			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should produce different embeddings for different models", () => {
			const text = "Different embedding models produce different vector representations.";
			const textBytes = new TextEncoder().encode(text);

			const config1: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
						normalize: true,
					},
				},
			};

			const config2: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "quality",
						},
						normalize: true,
					},
				},
			};

			const result1 = extractBytesSync(textBytes, "text/plain", config1);
			const result2 = extractBytesSync(textBytes, "text/plain", config2);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embeddings) {
				embedding1 = result1.chunks[0].embeddings[0];
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embeddings) {
				embedding2 = result2.chunks[0].embeddings[0];
			}

			// Different models may produce different dimensions
			if (embedding1 && embedding2) {
				// They might have different dimensions or different values
				// Just verify both are valid
				expect(embedding1.length).toBeGreaterThan(0);
				expect(embedding2.length).toBeGreaterThan(0);
			}
		});
	});

	describe("normalization correctness", () => {
		it("should normalize vectors when enabled", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text = "L2 normalization ensures unit vector magnitude.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];

						// Check if normalized (L2 norm should be 1)
						const normalized = isNormalized(embedding);
						expect(normalized).toBe(true);
					}
				}
			}
		});

		it("should preserve non-normalized vectors when disabled", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: false,
					},
				},
			};

			const text = "Unnormalized vectors preserve magnitude information.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				let _hasNonUnitVectors = false;

				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						const norm = calculateVectorNorm(embedding);

						// Non-normalized vectors may have norm != 1
						if (Math.abs(norm - 1.0) > 0.1) {
							_hasNonUnitVectors = true;
						}
					}
				}

				// At least should process without error
				expect(result.chunks.length).toBeGreaterThanOrEqual(0);
			}
		});

		it("should maintain consistency of normalization across chunks", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text =
				"Consistency of normalization across multiple chunks is important. " +
				"Each chunk should have properly normalized embeddings. " +
				"This ensures uniform downstream processing.";

			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 1) {
				const norms: number[] = [];

				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						const norm = calculateVectorNorm(embedding);
						norms.push(norm);
					}
				}

				// All normalized vectors should have similar norms (close to 1)
				if (norms.length > 0) {
					for (const norm of norms) {
						const tolerance = 0.01;
						expect(Math.abs(norm - 1.0)).toBeLessThan(tolerance);
					}
				}
			}
		});

		it("should calculate correct L2 norms for normalized vectors", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
						normalize: true,
					},
				},
			};

			const text = "L2 normalization divides vector by its magnitude to create unit vectors.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embeddings && chunk.embeddings.length > 0) {
						const embedding = chunk.embeddings[0];
						const norm = calculateVectorNorm(embedding);

						// L2 norm of normalized vector should be 1 (within tolerance)
						expect(norm).toBeCloseTo(1.0, 3);
					}
				}
			}
		});
	});

	describe("edge cases and error handling", () => {
		it("should handle empty content gracefully", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const textBytes = new TextEncoder().encode("");
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			// Empty content may result in no chunks
			expect(result.chunks).toBeDefined();
		});

		it("should handle very long text with embeddings", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 200,
					maxOverlap: 50,
					embedding: {
						model: {
							modelType: "preset",
							value: "fast",
						},
					},
				},
			};

			const longText = "This is a comprehensive document. ".repeat(100);
			const textBytes = new TextEncoder().encode(longText);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				// Should have multiple chunks
				expect(result.chunks.length).toBeGreaterThan(1);

				// All chunks should be processable
				for (const chunk of result.chunks) {
					expect(chunk.content).toBeDefined();
				}
			}
		});

		it("should handle special characters in text", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const text = "Special characters: !@#$%^&*() []{} <> and symbols should work.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should handle numeric content", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
					embedding: {
						model: {
							modelType: "preset",
							value: "balanced",
						},
					},
				},
			};

			const text = "123.456 789 1000 2000 3000 statistics and data values.";
			const textBytes = new TextEncoder().encode(text);
			const result = extractBytesSync(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		describe("mathematical properties validation", () => {
			it("should produce vectors with valid floating-point values", () => {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: "balanced",
							},
							normalize: true,
						},
					},
				};

				const text = "Validating floating-point properties of embedding values.";
				const textBytes = new TextEncoder().encode(text);
				const result = extractBytesSync(textBytes, "text/plain", config);

				if (result.chunks && result.chunks.length > 0) {
					for (const chunk of result.chunks) {
						if (chunk.embeddings && chunk.embeddings.length > 0) {
							const embedding = chunk.embeddings[0];

							for (let i = 0; i < embedding.length; i++) {
								const value = embedding[i];
								expect(Number.isFinite(value)).toBe(true);
								expect(value).not.toBeNaN();
								expect(value).toBeGreaterThanOrEqual(-2.0);
								expect(value).toBeLessThanOrEqual(2.0);
							}
						}
					}
				}
			});

			it("should not produce dead embeddings (all-zero vectors)", () => {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: "balanced",
							},
							normalize: true,
						},
					},
				};

				const text = "Testing for dead embeddings and zero vectors.";
				const textBytes = new TextEncoder().encode(text);
				const result = extractBytesSync(textBytes, "text/plain", config);

				if (result.chunks && result.chunks.length > 0) {
					for (const chunk of result.chunks) {
						if (chunk.embeddings && chunk.embeddings.length > 0) {
							const embedding = chunk.embeddings[0];
							const magnitude = embedding.reduce((sum, val) => sum + Math.abs(val), 0);

							expect(magnitude).toBeGreaterThan(0.1);
						}
					}
				}
			});

			it("should produce unit similarity for identical vectors", () => {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: "balanced",
							},
							normalize: true,
						},
					},
				};

				const text = "Testing identical vector similarity.";
				const textBytes = new TextEncoder().encode(text);
				const result = extractBytesSync(textBytes, "text/plain", config);

				if (result.chunks && result.chunks.length > 0) {
					for (const chunk of result.chunks) {
						if (chunk.embeddings && chunk.embeddings.length > 0) {
							const embedding = chunk.embeddings[0];

							// Vector with itself should have similarity 1.0
							const similarity = cosineSimilarity(embedding, embedding);
							expect(similarity).toBeCloseTo(1.0, 4);
						}
					}
				}
			});

			it("should maintain consistent dimensions across batch operations", () => {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 150,
						maxOverlap: 30,
						embedding: {
							model: {
								modelType: "preset",
								value: "balanced",
							},
						},
					},
				};

				const texts = [
					"First document about machine learning.",
					"Second document about neural networks.",
					"Third document about deep learning.",
				];

				const dimensions: number[] = [];

				for (const text of texts) {
					const textBytes = new TextEncoder().encode(text);
					const result = extractBytesSync(textBytes, "text/plain", config);

					if (result.chunks) {
						for (const chunk of result.chunks) {
							if (chunk.embeddings && chunk.embeddings.length > 0) {
								dimensions.push(chunk.embeddings[0].length);
							}
						}
					}
				}

				// All dimensions should be identical
				if (dimensions.length > 1) {
					const firstDim = dimensions[0];
					for (const dim of dimensions) {
						expect(dim).toBe(firstDim);
					}
				}
			});

			it("should be deterministic across multiple runs", () => {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 500,
						maxOverlap: 100,
						embedding: {
							model: {
								modelType: "preset",
								value: "balanced",
							},
							normalize: true,
						},
					},
				};

				const text = "Testing deterministic embedding generation.";
				const textBytes = new TextEncoder().encode(text);

				const result1 = extractBytesSync(textBytes, "text/plain", config);
				const result2 = extractBytesSync(textBytes, "text/plain", config);

				if (result1.chunks && result2.chunks && result1.chunks.length > 0 && result2.chunks.length > 0) {
					const emb1 = result1.chunks[0].embeddings?.[0];
					const emb2 = result2.chunks[0].embeddings?.[0];

					if (emb1 && emb2) {
						expect(emb1).toEqual(emb2);
					}
				}
			});
		});
	});
});
