```php
<?php

declare(strict_types=1);

/**
 * Chunking for RAG (Retrieval-Augmented Generation)
 *
 * Advanced chunking configuration optimized for RAG systems with embeddings.
 * Demonstrates how to process documents into chunks with embeddings for
 * vector database storage and semantic search.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Enums\EmbeddingModelType;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChars: 500,
        maxOverlap: 50,
        embedding: new EmbeddingConfig(
            model: EmbeddingModelType::preset('balanced'),
            normalize: true,
            batchSize: 16
        )
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('research_paper.pdf');

echo "RAG Chunking Results:\n";
echo str_repeat('=', 60) . "\n";

$chunksWithEmbeddings = [];
foreach ($result->chunks ?? [] as $chunk) {
    if ($chunk->embedding !== null) {
        $chunksWithEmbeddings[] = [
            'content' => substr($chunk->content, 0, 100) . '...',
            'embedding_dims' => count($chunk->embedding),
            'full_content' => $chunk->content,
            'embedding' => $chunk->embedding,
        ];
    }
}

echo "Chunks with embeddings: " . count($chunksWithEmbeddings) . "\n\n";

echo "Sample chunks for vector database:\n";
echo str_repeat('=', 60) . "\n";

foreach (array_slice($chunksWithEmbeddings, 0, 3) as $index => $chunk) {
    echo "Chunk " . ($index + 1) . ":\n";
    echo "  Content preview: {$chunk['content']}\n";
    echo "  Embedding dimensions: {$chunk['embedding_dims']}\n";
    echo "  Ready for vector DB: Yes\n\n";
}

$vectorDbRecords = array_map(
    fn($chunk, $idx) => [
        'id' => sprintf('doc_%s_chunk_%d', md5('research_paper.pdf'), $idx),
        'content' => $chunk['full_content'],
        'embedding' => $chunk['embedding'],
        'metadata' => [
            'source' => 'research_paper.pdf',
            'chunk_index' => $idx,
            'char_count' => strlen($chunk['full_content']),
        ],
    ],
    $chunksWithEmbeddings,
    array_keys($chunksWithEmbeddings)
);

echo "Prepared " . count($vectorDbRecords) . " records for vector database\n";
echo "Each record contains: id, content, embedding, and metadata\n";
```
