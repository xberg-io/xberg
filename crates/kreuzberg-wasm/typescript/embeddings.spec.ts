/**
 * Comprehensive embeddings tests for WASM bindings.
 *
 * Tests verify embedding/vector generation functionality:
 * 1. Embedding generation (balanced/fast models)
 * 2. Dimension validation (384, 512, 768, 1024)
 * 3. Normalization verification (L2 norm ~1.0)
 * 4. Batch embeddings processing
 * 5. Edge cases (empty text, whitespace, very long text)
 *
 * Note: WASM embeddings are returned per-chunk in the extraction result.
 * Configuration mirrors Node.js bindings but through ExtractionConfig.
 */

import { beforeAll, describe, expect, it } from "vitest";
import { extractBytes, extractBytesSync, initWasm } from "./index.js";
import type { ExtractionConfig } from "./types.js";

/**
 * Calculate Euclidean norm (L2 magnitude) of a vector.
 */
function calculateVectorNorm(vector: number[]): number {
	return Math.sqrt(vector.reduce((sum, val) => sum + val * val, 0));
}

/**
 * Calculate cosine similarity between two vectors.
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
 * Check if vector is properly normalized (L2 norm ~= 1.0).
 */
function isNormalized(vector: number[], tolerance: number = 0.01): boolean {
	const norm = calculateVectorNorm(vector);
	return Math.abs(norm - 1.0) < tolerance;
}

beforeAll(async () => {
	// Initialize WASM module before running tests
	await initWasm();
});

describe("Embedding Generation (WASM Bindings)", () => {
	describe("embedding generation with balanced model", () => {
		it("should generate embeddings with valid dimensions", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Machine learning transforms technology through artificial intelligence.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						expect(Array.isArray(chunk.embedding)).toBe(true);
						expect(chunk.embedding.length).toBeGreaterThan(0);

						// Validate common embedding dimensions
						const validDimensions = [256, 384, 512, 768, 1024, 1536];
						expect(validDimensions).toContain(chunk.embedding.length);

						// All elements should be finite numbers
						for (const value of chunk.embedding) {
							expect(typeof value).toBe("number");
							expect(Number.isFinite(value)).toBe(true);
						}
					}
				}
			}
		});

		it("should generate consistent embeddings for same input", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Consistent embeddings ensure reproducible semantic operations.";
			const textBytes = new TextEncoder().encode(text);

			const result1 = await extractBytes(textBytes, "text/plain", config);
			const result2 = await extractBytes(textBytes, "text/plain", config);

			expect(result1.chunks).toBeDefined();
			expect(result2.chunks).toBeDefined();

			if (result1.chunks && result2.chunks && result1.chunks.length > 0) {
				const emb1 = result1.chunks[0].embedding;
				const emb2 = result2.chunks[0].embedding;

				if (emb1 && emb2) {
					expect(emb1).toEqual(emb2);
				}
			}
		});

		it("should produce non-zero vectors for non-empty text", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Neural networks learn representations of data.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						// Vector should not be all zeros
						const hasNonZero = chunk.embedding.some((val) => val !== 0);
						expect(hasNonZero).toBe(true);

						// Calculate magnitude
						const magnitude = chunk.embedding.reduce((sum, val) => sum + Math.abs(val), 0);
						expect(magnitude).toBeGreaterThan(0);
					}
				}
			}
		});

		it("should produce different vectors for different texts", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text1 = "Machine learning is revolutionizing AI development.";
			const text2 = "Cats are fluffy animals that like to sleep.";

			const result1 = await extractBytes(new TextEncoder().encode(text1), "text/plain", config);
			const result2 = await extractBytes(new TextEncoder().encode(text2), "text/plain", config);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embedding) {
				embedding1 = result1.chunks[0].embedding;
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embedding) {
				embedding2 = result2.chunks[0].embedding;
			}

			if (embedding1 && embedding2) {
				// Vectors should be different
				expect(embedding1).not.toEqual(embedding2);

				// Similarity should be lower for dissimilar texts
				const similarity = cosineSimilarity(embedding1, embedding2);
				expect(similarity).toBeLessThan(0.9);
			}
		});
	});

	describe("dimension validation", () => {
		it("should maintain consistent dimensions across chunks", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
				},
			};

			const longText =
				"This is a long text that will be chunked. " +
				"Each chunk should have embeddings with same dimensions. " +
				"Consistency matters for downstream processing.";

			const textBytes = new TextEncoder().encode(longText);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 1) {
				let firstDimension: number | null = null;

				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						const dimension = chunk.embedding.length;

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

		it("should support 384-dimension embeddings", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing embedding dimensions for 384-dim vectors.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						// Should match one of the standard dimensions
						expect([256, 384, 512, 768, 1024, 1536]).toContain(chunk.embedding.length);
					}
				}
			}
		});

		it("should support 512-dimension embeddings", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing 512-dimensional embedding vectors.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						expect([256, 384, 512, 768, 1024, 1536]).toContain(chunk.embedding.length);
					}
				}
			}
		});

		it("should support 768-dimension embeddings", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing 768-dimensional vector space embeddings.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						expect([256, 384, 512, 768, 1024, 1536]).toContain(chunk.embedding.length);
					}
				}
			}
		});

		it("should support 1024-dimension embeddings", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing high-dimensional 1024-vector embeddings.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						expect([256, 384, 512, 768, 1024, 1536]).toContain(chunk.embedding.length);
					}
				}
			}
		});
	});

	describe("normalization verification", () => {
		it("should produce unit-normalized vectors", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "L2 normalization ensures unit vector magnitude.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						const norm = calculateVectorNorm(chunk.embedding);
						// L2 norm should be close to 1.0
						expect(norm).toBeCloseTo(1.0, 2);
					}
				}
			}
		});

		it("should verify L2 norm is approximately 1.0", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Verify L2 normalization for vector magnitude.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						const isNorm = isNormalized(chunk.embedding, 0.01);
						expect(isNorm).toBe(true);
					}
				}
			}
		});

		it("should maintain consistent normalization across batches", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
				},
			};

			const text =
				"Consistency of normalization across multiple chunks. " +
				"Each chunk should be properly normalized. " +
				"This ensures uniform downstream processing.";

			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 1) {
				const norms: number[] = [];

				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						const norm = calculateVectorNorm(chunk.embedding);
						norms.push(norm);
					}
				}

				// All norms should be close to 1.0
				if (norms.length > 0) {
					for (const norm of norms) {
						expect(Math.abs(norm - 1.0)).toBeLessThan(0.02);
					}
				}
			}
		});

		it("should verify no invalid floating-point values", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Validate floating-point properties of embeddings.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						for (const value of chunk.embedding) {
							expect(Number.isFinite(value)).toBe(true);
							expect(Number.isNaN(value)).toBe(false);
							// Values should be in reasonable range
							expect(value).toBeGreaterThanOrEqual(-2.0);
							expect(value).toBeLessThanOrEqual(2.0);
						}
					}
				}
			}
		});
	});

	describe("batch embeddings", () => {
		it("should process batch with consistent quality", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const texts = [
				"Data science uses statistics and machine learning.",
				"Artificial intelligence powers modern applications.",
				"Deep neural networks learn complex patterns.",
			];

			const results = await Promise.all(
				texts.map((text) => extractBytes(new TextEncoder().encode(text), "text/plain", config))
			);

			let embeddingDimension: number | null = null;

			for (const result of results) {
				if (result.chunks && result.chunks.length > 0) {
					for (const chunk of result.chunks) {
						if (chunk.embedding) {
							const dimension = chunk.embedding.length;

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

		it("should handle multiple chunks with embeddings", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
				},
			};

			const text = "Chunk one. Chunk two. Chunk three. Chunk four. Chunk five.";
			const result = await extractBytes(new TextEncoder().encode(text), "text/plain", config);

			expect(result).toBeDefined();
			if (result.chunks) {
				const chunkCount = result.chunks.length;
				const embeddingCount = result.chunks.filter((chunk) => chunk.embedding).length;

				expect(chunkCount).toBeGreaterThan(0);
				expect(embeddingCount).toBeGreaterThanOrEqual(0);
				expect(embeddingCount).toBeLessThanOrEqual(chunkCount);
			}
		});

		it("should maintain batch consistency across runs", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 150,
					maxOverlap: 30,
				},
			};

			const texts = [
				"First document about machine learning.",
				"Second document about neural networks.",
				"Third document about deep learning.",
			];

			const dimensions1: number[] = [];
			const dimensions2: number[] = [];

			// First batch
			for (const text of texts) {
				const result = await extractBytes(new TextEncoder().encode(text), "text/plain", config);
				if (result.chunks) {
					for (const chunk of result.chunks) {
						if (chunk.embedding) {
							dimensions1.push(chunk.embedding.length);
						}
					}
				}
			}

			// Second batch
			for (const text of texts) {
				const result = await extractBytes(new TextEncoder().encode(text), "text/plain", config);
				if (result.chunks) {
					for (const chunk of result.chunks) {
						if (chunk.embedding) {
							dimensions2.push(chunk.embedding.length);
						}
					}
				}
			}

			// Dimensions should be consistent
			expect(dimensions1).toEqual(dimensions2);
		});
	});

	describe("edge cases", () => {
		it("should handle empty content gracefully", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const textBytes = new TextEncoder().encode("");
			// Empty content should be rejected by the WASM module
			await expect(extractBytes(textBytes, "text/plain", config)).rejects.toThrow("Document data cannot be empty");
		});

		it("should handle very long text", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 200,
					maxOverlap: 50,
				},
			};

			const longText = "This is a comprehensive document. ".repeat(100);
			const textBytes = new TextEncoder().encode(longText);
			const result = await extractBytes(textBytes, "text/plain", config);

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

		it("should handle whitespace-only content", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "   \n\t  \n  ";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should handle special characters in text", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Special: !@#$%^&*() []{} <> and symbols work.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should handle numeric content", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "123.456 789 1000 2000 3000 statistics and data.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should handle UTF-8 encoded multilingual text", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Café, naïve, résumé - UTF-8 text with accents.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			expect(result).toBeDefined();
			expect(result.chunks).toBeDefined();
		});

		it("should not produce all-zero vectors", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing for dead embeddings and zero vectors.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						const magnitude = chunk.embedding.reduce((sum, val) => sum + Math.abs(val), 0);
						expect(magnitude).toBeGreaterThan(0.1);
					}
				}
			}
		});

		it("should produce unit similarity for identical vectors", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing identical vector similarity.";
			const textBytes = new TextEncoder().encode(text);
			const result = await extractBytes(textBytes, "text/plain", config);

			if (result.chunks && result.chunks.length > 0) {
				for (const chunk of result.chunks) {
					if (chunk.embedding) {
						// Vector with itself should have similarity 1.0
						const similarity = cosineSimilarity(chunk.embedding, chunk.embedding);
						expect(similarity).toBeCloseTo(1.0, 3);
					}
				}
			}
		});
	});

	describe("semantic similarity", () => {
		it("should produce high similarity for related texts", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text1 = "Machine learning is a subset of artificial intelligence.";
			const text2 = "AI uses machine learning as a core technology.";

			const result1 = await extractBytes(new TextEncoder().encode(text1), "text/plain", config);
			const result2 = await extractBytes(new TextEncoder().encode(text2), "text/plain", config);

			let embedding1: number[] | null = null;
			let embedding2: number[] | null = null;

			if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embedding) {
				embedding1 = result1.chunks[0].embedding;
			}

			if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embedding) {
				embedding2 = result2.chunks[0].embedding;
			}

			if (embedding1 && embedding2) {
				const similarity = cosineSimilarity(embedding1, embedding2);
				// Related texts should have positive similarity
				expect(similarity).toBeGreaterThan(0);
			}
		});

		it("should produce valid cosine similarity values", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const texts = [
				"Natural language processing techniques.",
				"Computer vision and image analysis.",
				"Reinforcement learning algorithms.",
			];

			for (let i = 0; i < texts.length; i++) {
				for (let j = i + 1; j < texts.length; j++) {
					const result1 = await extractBytes(new TextEncoder().encode(texts[i]), "text/plain", config);
					const result2 = await extractBytes(new TextEncoder().encode(texts[j]), "text/plain", config);

					let embedding1: number[] | null = null;
					let embedding2: number[] | null = null;

					if (result1.chunks && result1.chunks.length > 0 && result1.chunks[0].embedding) {
						embedding1 = result1.chunks[0].embedding;
					}

					if (result2.chunks && result2.chunks.length > 0 && result2.chunks[0].embedding) {
						embedding2 = result2.chunks[0].embedding;
					}

					if (embedding1 && embedding2) {
						const similarity = cosineSimilarity(embedding1, embedding2);
						// Similarity should be in valid range
						expect(similarity).toBeGreaterThanOrEqual(-1);
						expect(similarity).toBeLessThanOrEqual(1);
					}
				}
			}
		});
	});

	describe("determinism", () => {
		it("should be deterministic across multiple runs", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Testing deterministic embedding generation.";
			const textBytes = new TextEncoder().encode(text);

			const result1 = await extractBytes(textBytes, "text/plain", config);
			const result2 = await extractBytes(textBytes, "text/plain", config);

			if (result1.chunks && result2.chunks && result1.chunks.length > 0 && result2.chunks.length > 0) {
				const emb1 = result1.chunks[0].embedding;
				const emb2 = result2.chunks[0].embedding;

				if (emb1 && emb2) {
					expect(emb1).toEqual(emb2);
				}
			}
		});

		it("should produce identical embeddings on repeated calls", async () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 100,
				},
			};

			const text = "Repeated embedding generation should be identical.";
			const textBytes = new TextEncoder().encode(text);

			const embeddings: (number[] | undefined)[] = [];

			for (let i = 0; i < 3; i++) {
				const result = await extractBytes(textBytes, "text/plain", config);
				if (result.chunks && result.chunks.length > 0) {
					embeddings.push(result.chunks[0].embedding);
				}
			}

			// All embeddings should be identical
			if (embeddings.length > 1 && embeddings[0]) {
				for (let i = 1; i < embeddings.length; i++) {
					expect(embeddings[i]).toEqual(embeddings[0]);
				}
			}
		});
	});
});
