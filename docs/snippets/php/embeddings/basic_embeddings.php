```php
<?php

declare(strict_types=1);

/**
 * Basic Embedding Generation
 *
 * Generate vector embeddings for semantic search and similarity matching.
 * Requires ONNX Runtime to be installed.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50
    ),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "Embedding Generation Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Chunks with embeddings: " . count($result->chunks ?? []) . "\n\n";

foreach ($result->chunks ?? [] as $chunk) {
    echo "Chunk {$chunk->metadata->chunkIndex}:\n";
    echo "  Content length: " . strlen($chunk->content) . " chars\n";

    if ($chunk->embedding !== null) {
        echo "  Embedding dimension: " . count($chunk->embedding) . "\n";
        echo "  First 5 values: [" . implode(', ', array_map(
            fn($v) => number_format($v, 4),
            array_slice($chunk->embedding, 0, 5)
        )) . "...]\n";
    }
    echo "\n";
}

$models = [
    'all-MiniLM-L6-v2',      
    'all-mpnet-base-v2',     
    'paraphrase-multilingual-MiniLM-L12-v2', 
];

foreach ($models as $model) {
    echo "Testing model: $model\n";

    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(maxChunkSize: 256),
        embedding: new EmbeddingConfig(
            model: $model,
            normalize: true
        )
    );

    $kreuzberg = new Kreuzberg($config);
    $start = microtime(true);
    $result = $kreuzberg->extractFile('test_doc.pdf');
    $elapsed = microtime(true) - $start;

    $chunk = ($result->chunks ?? [])[0] ?? null;
    if ($chunk && $chunk->embedding) {
        echo "  Dimension: " . count($chunk->embedding) . "\n";
        echo "  Time: " . number_format($elapsed, 3) . "s\n";
        echo "  Chunks: " . count($result->chunks ?? []) . "\n\n";
    }
}

function cosineSimilarity(array $a, array $b): float
{
    $dotProduct = 0.0;
    $magnitudeA = 0.0;
    $magnitudeB = 0.0;

    for ($i = 0; $i < count($a); $i++) {
        $dotProduct += $a[$i] * $b[$i];
        $magnitudeA += $a[$i] * $a[$i];
        $magnitudeB += $b[$i] * $b[$i];
    }

    return $dotProduct / (sqrt($magnitudeA) * sqrt($magnitudeB));
}

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(model: 'all-MiniLM-L6-v2', normalize: true)
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "Chunk Similarity Analysis:\n";
echo str_repeat('=', 60) . "\n";

$chunks = $result->chunks ?? [];
if (count($chunks) >= 2) {
    $referenceChunk = $chunks[0];

    foreach (array_slice($chunks, 1, 5) as $chunk) {
        if ($referenceChunk->embedding && $chunk->embedding) {
            $similarity = cosineSimilarity(
                $referenceChunk->embedding,
                $chunk->embedding
            );

            echo "Chunk 0 vs Chunk {$chunk->metadata->chunkIndex}: ";
            echo number_format($similarity, 4) . "\n";
        }
    }
}
echo "\n";

class SimpleVectorDB
{
    private array $vectors = [];

    public function add(string $id, array $embedding, string $content): void
    {
        $this->vectors[$id] = [
            'embedding' => $embedding,
            'content' => $content,
        ];
    }

    public function search(array $queryEmbedding, int $k = 5): array
    {
        $results = [];

        foreach ($this->vectors as $id => $data) {
            $similarity = $this->cosineSimilarity($queryEmbedding, $data['embedding']);
            $results[] = [
                'id' => $id,
                'similarity' => $similarity,
                'content' => $data['content'],
            ];
        }

        usort($results, fn($a, $b) => $b['similarity'] <=> $a['similarity']);

        return array_slice($results, 0, $k);
    }

    private function cosineSimilarity(array $a, array $b): float
    {
        $dotProduct = 0.0;
        $magA = 0.0;
        $magB = 0.0;

        for ($i = 0; $i < count($a); $i++) {
            $dotProduct += $a[$i] * $b[$i];
            $magA += $a[$i] * $a[$i];
            $magB += $b[$i] * $b[$i];
        }

        return $dotProduct / (sqrt($magA) * sqrt($magB));
    }
}

$db = new SimpleVectorDB();

$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
foreach ($files as $file) {
    if (!file_exists($file)) continue;

    $result = $kreuzberg->extractFile($file);

    foreach ($result->chunks ?? [] as $chunk) {
        if ($chunk->embedding) {
            $id = $file . '_chunk_' . $chunk->metadata->chunkIndex;
            $db->add($id, $chunk->embedding, $chunk->content);
        }
    }
}

echo "Vector database built\n";
echo "Ready for semantic search!\n";

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 512),
    embedding: new EmbeddingConfig(model: 'all-MiniLM-L6-v2', normalize: true)
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('export_doc.pdf');

$exportData = [];
foreach ($result->chunks ?? [] as $chunk) {
    $exportData[] = [
        'id' => uniqid('vec_', true),
        'text' => $chunk->content,
        'embedding' => $chunk->embedding,
        'metadata' => [
            'chunk_index' => $chunk->metadata->chunkIndex,
            'source' => 'export_doc.pdf',
            'timestamp' => time(),
        ],
    ];
}

file_put_contents('embeddings_export.json', json_encode($exportData));
echo "\nExported " . count($exportData) . " embeddings to embeddings_export.json\n";
```
