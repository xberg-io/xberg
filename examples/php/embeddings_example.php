<?php

declare(strict_types=1);

/**
 * Embeddings Example
 *
 * Demonstrates generating embeddings for semantic search and RAG applications.
 * Shows how to combine text chunking with embedding generation.
 *
 * This example covers:
 * - Basic embedding generation
 * - Chunking with embeddings
 * - Different embedding models
 * - Embedding normalization
 * - Using embeddings for semantic search
 * - Batch size configuration
 * - Cosine similarity calculation
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_file;


echo "=== Example 1: Basic Embedding Generation ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    echo "Embedding generation results:\n";
    echo "  Total chunks: " . count($result->chunks ?? []) . "\n";

    if ($result->chunks !== null && count($result->chunks) > 0) {
        $firstChunk = $result->chunks[0];

        echo "\nFirst chunk:\n";
        echo "  Content length: " . strlen($firstChunk->content) . " characters\n";
        echo "  Has embedding: " . ($firstChunk->embedding !== null ? 'Yes' : 'No') . "\n";

        if ($firstChunk->embedding !== null) {
            echo "  Embedding dimension: " . count($firstChunk->embedding) . "\n";
            echo "  First 5 values: " . implode(', ', array_map(
                static fn ($v) => round($v, 4),
                array_slice($firstChunk->embedding, 0, 5)
            )) . "...\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Different Embedding Models ===\n\n";

try {

    $models = [
        'all-MiniLM-L6-v2' => 384,
    ];

    foreach ($models as $modelName => $expectedDim) {
        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(maxChunkSize: 256, chunkOverlap: 25),
            embedding: new EmbeddingConfig(
                model: $modelName,
                normalize: true,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

        if ($result->chunks !== null && count($result->chunks) > 0) {
            $embeddingDim = count($result->chunks[0]->embedding ?? []);

            echo "Model: {$modelName}\n";
            echo "  Chunks: " . count($result->chunks) . "\n";
            echo "  Embedding dimension: {$embeddingDim}\n";
            echo "  Expected dimension: {$expectedDim}\n\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Normalized vs Non-Normalized Embeddings ===\n\n";

try {
    $filePath = __DIR__ . '/../sample-documents/document.pdf';

    $config1 = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: false,
        ),
    );

    $kreuzberg1 = new Kreuzberg($config1);
    $result1 = $kreuzberg1->extractFile($filePath);

    $config2 = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg2 = new Kreuzberg($config2);
    $result2 = $kreuzberg2->extractFile($filePath);

    if ($result1->chunks !== null && $result1->chunks[0]->embedding !== null &&
        $result2->chunks !== null && $result2->chunks[0]->embedding !== null) {

        $norm1 = sqrt(array_sum(array_map(
            static fn ($v) => $v * $v,
            $result1->chunks[0]->embedding
        )));

        $norm2 = sqrt(array_sum(array_map(
            static fn ($v) => $v * $v,
            $result2->chunks[0]->embedding
        )));

        echo "Non-normalized embeddings:\n";
        echo "  L2 norm: " . round($norm1, 4) . "\n";
        echo "  First value: " . round($result1->chunks[0]->embedding[0], 4) . "\n\n";

        echo "Normalized embeddings:\n";
        echo "  L2 norm: " . round($norm2, 4) . " (should be ~1.0)\n";
        echo "  First value: " . round($result2->chunks[0]->embedding[0], 4) . "\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Embedding Batch Size Configuration ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
            batchSize: 32,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/large_document.pdf');

    echo "Batch embedding generation:\n";
    echo "  Batch size: 32\n";
    echo "  Total chunks: " . count($result->chunks ?? []) . "\n";
    echo "  Note: Larger batch sizes can improve performance\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: Cosine Similarity Calculation ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null && count($result->chunks) >= 3) {
        echo "Cosine similarity between chunks:\n\n";

        $cosineSimilarity = function (array $vec1, array $vec2): float {
            $dotProduct = 0.0;
            for ($i = 0; $i < count($vec1); $i++) {
                $dotProduct += $vec1[$i] * $vec2[$i];
            }

            return $dotProduct;
        };

        $chunk0 = $result->chunks[0];
        $chunk1 = $result->chunks[1];
        $chunk2 = $result->chunks[2];

        if ($chunk0->embedding !== null && $chunk1->embedding !== null && $chunk2->embedding !== null) {
            $sim01 = $cosineSimilarity($chunk0->embedding, $chunk1->embedding);
            $sim02 = $cosineSimilarity($chunk0->embedding, $chunk2->embedding);

            echo "Chunk 0 vs Chunk 1: " . round($sim01, 4) . "\n";
            echo "Chunk 0 vs Chunk 2: " . round($sim02, 4) . "\n";
            echo "\nNote: Values closer to 1.0 indicate higher similarity\n\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Semantic Search Example ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null && count($result->chunks) > 0) {
        echo "Semantic search setup:\n";
        echo "  Total chunks indexed: " . count($result->chunks) . "\n";
        echo "  Ready for similarity search\n\n";


        echo "Example workflow:\n";
        echo "  1. User query: 'What is machine learning?'\n";
        echo "  2. Generate query embedding\n";
        echo "  3. Calculate similarity with all chunk embeddings\n";
        echo "  4. Return top-k most similar chunks\n";
        echo "  5. Use chunks as context for LLM response\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Procedural API for Embeddings ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $result = extract_file(
        __DIR__ . '/../sample-documents/article.pdf',
        config: $config
    );

    echo "Procedural API embeddings:\n";
    if ($result->chunks !== null) {
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  First chunk has embedding: " . (
            $result->chunks[0]->embedding !== null ? 'Yes' : 'No'
        ) . "\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 8: Embedding Statistics ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null && count($result->chunks) > 0) {
        echo "Embedding statistics:\n";
        echo "  Total chunks: " . count($result->chunks) . "\n";

        $embeddedChunks = array_filter(
            $result->chunks,
            static fn ($chunk) => $chunk->embedding !== null
        );

        echo "  Chunks with embeddings: " . count($embeddedChunks) . "\n";

        if (count($embeddedChunks) > 0) {
            $firstEmbedding = reset($embeddedChunks)->embedding;
            if ($firstEmbedding !== null) {
                echo "  Embedding dimension: " . count($firstEmbedding) . "\n";

                $allValues = [];
                foreach ($embeddedChunks as $chunk) {
                    if ($chunk->embedding !== null) {
                        $allValues = array_merge($allValues, $chunk->embedding);
                    }
                }

                echo "  Value range: [" . round(min($allValues), 4) . ", " .
                    round(max($allValues), 4) . "]\n";
                echo "  Mean value: " . round(
                    array_sum($allValues) / count($allValues),
                    4
                ) . "\n";
            }
        }

        echo "\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 9: Building a Vector Database ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 512, chunkOverlap: 50),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null) {
        echo "Preparing data for vector database:\n\n";

        $vectorDB = [];

        foreach ($result->chunks as $chunk) {
            if ($chunk->embedding !== null) {
                $vectorDB[] = [
                    'id' => $chunk->metadata->chunkIndex,
                    'content' => $chunk->content,
                    'embedding' => $chunk->embedding,
                    'metadata' => [
                        'byte_start' => $chunk->metadata->byteStart,
                        'byte_end' => $chunk->metadata->byteEnd,
                        'token_count' => $chunk->metadata->tokenCount,
                        'first_page' => $chunk->metadata->firstPage,
                        'last_page' => $chunk->metadata->lastPage,
                    ],
                ];
            }
        }

        echo "Vector database entries: " . count($vectorDB) . "\n";
        echo "\nExample entry structure:\n";
        if (count($vectorDB) > 0) {
            $example = $vectorDB[0];
            echo "  ID: {$example['id']}\n";
            echo "  Content length: " . strlen($example['content']) . " characters\n";
            echo "  Embedding dimension: " . count($example['embedding']) . "\n";
            echo "  Metadata keys: " . implode(', ', array_keys($example['metadata'])) . "\n";
        }

        echo "\nNote: In production, use a vector database like:\n";
        echo "  - Pinecone\n";
        echo "  - Weaviate\n";
        echo "  - Qdrant\n";
        echo "  - Milvus\n";
        echo "  - ChromaDB\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 10: RAG Pipeline Example ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null) {
        echo "RAG (Retrieval-Augmented Generation) Pipeline:\n\n";

        echo "Step 1: Document Processing\n";
        echo "  - Extract text from document: OK\n";
        echo "  - Split into chunks: " . count($result->chunks) . " chunks\n";
        echo "  - Generate embeddings: OK\n\n";

        echo "Step 2: Indexing\n";
        echo "  - Store chunks and embeddings in vector database\n";
        echo "  - Create metadata index for filtering\n\n";

        echo "Step 3: Query Processing (example)\n";
        echo "  - User query: 'Explain the main concepts'\n";
        echo "  - Generate query embedding\n";
        echo "  - Find top-k similar chunks (k=3)\n\n";

        echo "Step 4: Context Retrieval\n";
        echo "  - Retrieved chunks provide context\n";
        echo "  - Include metadata (page numbers, etc.)\n\n";

        echo "Step 5: LLM Generation\n";
        echo "  - Send query + retrieved context to LLM\n";
        echo "  - Generate answer based on document content\n";
        echo "  - Include source citations\n\n";

        echo "Benefits of this approach:\n";
        echo "  - Grounded responses (no hallucination)\n";
        echo "  - Source attribution\n";
        echo "  - Works with documents larger than context window\n";
        echo "  - Fast retrieval with vector search\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "Done!\n";
