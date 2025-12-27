```php
<?php

declare(strict_types=1);

/**
 * Embedding Generation Configuration
 *
 * This example demonstrates how to configure embedding generation for semantic search
 * and vector database applications.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;

echo "Example 1: Basic Embedding Generation\n";
echo "=====================================\n";

$config1 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50
    ),
    embedding: new EmbeddingConfig()  
);

$kreuzberg = new Kreuzberg($config1);
$result = $kreuzberg->extractFile('document.pdf');

if ($result->chunks !== null) {
    foreach ($result->chunks as $i => $chunk) {
        echo "\nChunk {$i}:\n";
        echo "- Text: " . substr($chunk->text, 0, 50) . "...\n";
        if ($chunk->embedding !== null) {
            echo "- Embedding dimension: " . count($chunk->embedding) . "\n";
            echo "- First 5 values: [" . implode(', ', array_slice($chunk->embedding, 0, 5)) . "...]\n";
        }
    }
}

echo "\n\n";

echo "Example 2: Different Embedding Models\n";
echo "====================================\n";

$config2a = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',  
        normalize: true,
        batchSize: 32
    )
);

echo "Model: all-MiniLM-L6-v2\n";
echo "- Dimensions: 384\n";
echo "- Speed: Very Fast\n";
echo "- Use case: General purpose, quick retrieval\n\n";

$config2b = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-mpnet-base-v2',  
        normalize: true,
        batchSize: 16  
    )
);

echo "Model: all-mpnet-base-v2\n";
echo "- Dimensions: 768\n";
echo "- Speed: Medium\n";
echo "- Use case: Higher quality semantic search\n\n";

echo "Example 3: Normalized vs Non-Normalized Embeddings\n";
echo "==================================================\n";

$config3a = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true  
    )
);

echo "Normalized embeddings:\n";
echo "- Better for cosine similarity\n";
echo "- Values in range [-1, 1]\n";
echo "- Faster similarity computation\n\n";

$config3b = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: false  
    )
);

echo "Non-normalized embeddings:\n";
echo "- Raw model output\n";
echo "- Useful for specific distance metrics\n\n";

echo "Example 4: Batch Size Configuration\n";
echo "===================================\n";

$config4a = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true,
        batchSize: 8  
    )
);

echo "Batch size: 8\n";
echo "- Lower memory usage\n";
echo "- Slower processing\n";
echo "- Good for limited resources\n\n";

$config4b = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true,
        batchSize: 64  
    )
);

echo "Batch size: 64\n";
echo "- Higher memory usage\n";
echo "- Faster processing\n";
echo "- Good for high-performance systems\n\n";

echo "Example 5: Complete RAG Pipeline\n";
echo "================================\n";

$config5 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true
    ),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true,
        batchSize: 32
    )
);

$result5 = (new Kreuzberg($config5))->extractFile('document.pdf');

if ($result5->chunks !== null) {
    echo "Processing " . count($result5->chunks) . " chunks with embeddings...\n\n";

    $vectorDbData = [];
    foreach ($result5->chunks as $i => $chunk) {
        if ($chunk->embedding !== null) {
            $vectorDbData[] = [
                'id' => "chunk_{$i}",
                'text' => $chunk->text,
                'embedding' => $chunk->embedding,
                'metadata' => [
                    'char_count' => $chunk->metadata->charCount,
                    'page_range' => $chunk->metadata->firstPage !== null
                        ? "{$chunk->metadata->firstPage}-{$chunk->metadata->lastPage}"
                        : null,
                ],
            ];
        }
    }

    echo "Prepared " . count($vectorDbData) . " vectors for database\n";
    echo "Each vector has " . count($vectorDbData[0]['embedding']) . " dimensions\n";
}

echo "\n\nEmbedding Configuration Parameters:\n";
echo "===================================\n";
echo "- model: Embedding model name\n";
echo "  * 'all-MiniLM-L6-v2': 384 dims, fast, general purpose\n";
echo "  * 'all-mpnet-base-v2': 768 dims, higher quality\n";
echo "- normalize: L2 normalize embeddings (recommended: true)\n";
echo "- batchSize: Number of chunks to process at once\n";
echo "\nBest Practices:\n";
echo "- Use normalized embeddings for cosine similarity\n";
echo "- Choose batch size based on available memory\n";
echo "- Use all-MiniLM-L6-v2 for speed, all-mpnet-base-v2 for quality\n";
echo "- Combine with chunking for optimal RAG performance\n";

echo "\n\nCommon Embedding Models:\n";
echo "========================\n";
echo "Model                     | Dimensions | Speed    | Use Case\n";
echo "--------------------------|------------|----------|---------------------------\n";
echo "all-MiniLM-L6-v2         | 384        | Fast     | General purpose, QA\n";
echo "all-mpnet-base-v2        | 768        | Medium   | Better semantic search\n";
echo "paraphrase-MiniLM-L6-v2  | 384        | Fast     | Paraphrase detection\n";
echo "paraphrase-mpnet-base-v2 | 768        | Medium   | High-quality paraphrase\n";
```
