```php
<?php

declare(strict_types=1);

/**
 * Embedding Generation with Chunking
 *
 * Configure chunking with automatic embedding generation for each chunk.
 * Ideal for semantic search, similarity matching, and vector databases.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Enums\EmbeddingModelType;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChars: 1024,
        maxOverlap: 100,
        embedding: new EmbeddingConfig(
            model: EmbeddingModelType::preset('balanced'),
            normalize: true,
            batchSize: 32,
            showDownloadProgress: false
        )
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "Embedding Generation Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Total chunks: " . count($result->chunks ?? []) . "\n\n";

$chunksWithEmbeddings = 0;
$totalEmbeddingDimensions = 0;

foreach ($result->chunks ?? [] as $chunk) {
    if ($chunk->embedding !== null) {
        $chunksWithEmbeddings++;
        $totalEmbeddingDimensions = count($chunk->embedding);
    }
}

echo "Chunks with embeddings: $chunksWithEmbeddings\n";
echo "Embedding dimensions: $totalEmbeddingDimensions\n";
echo "Coverage: " . ($chunksWithEmbeddings > 0
    ? sprintf("%.1f%%", ($chunksWithEmbeddings / count($result->chunks ?? [1])) * 100)
    : "0%") . "\n\n";

if (!empty($result->chunks) && $result->chunks[0]->embedding !== null) {
    $sampleChunk = $result->chunks[0];

    echo "Sample Chunk:\n";
    echo str_repeat('=', 60) . "\n";
    echo "Content preview: " . substr($sampleChunk->content, 0, 150) . "...\n";
    echo "Content length: " . strlen($sampleChunk->content) . " chars\n";
    echo "Embedding dimensions: " . count($sampleChunk->embedding) . "\n";
    echo "First 5 embedding values: [";
    echo implode(', ', array_map(
        fn($v) => number_format($v, 4),
        array_slice($sampleChunk->embedding, 0, 5)
    ));
    echo ", ...]\n\n";
}

if (!empty($result->chunks)) {
    $totalChars = array_sum(array_map(
        fn($chunk) => strlen($chunk->content),
        $result->chunks
    ));
    $avgChunkSize = $totalChars / count($result->chunks);

    echo "Average chunk size: " . round($avgChunkSize) . " characters\n";
}
```
