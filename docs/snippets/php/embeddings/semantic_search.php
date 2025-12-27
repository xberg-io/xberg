```php
<?php

declare(strict_types=1);

/**
 * Semantic Search with Embeddings
 *
 * Build a semantic search system using document embeddings.
 * Find relevant content based on meaning, not just keywords.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true
    ),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true
    )
);

$kreuzberg = new Kreuzberg($config);

echo "Building document index...\n";
$documentIndex = [];

$files = glob('knowledge_base/*.pdf');
foreach ($files as $file) {
    echo "Indexing: " . basename($file) . "\n";

    $result = $kreuzberg->extractFile($file);

    foreach ($result->chunks ?? [] as $chunk) {
        if ($chunk->embedding) {
            $documentIndex[] = [
                'file' => basename($file),
                'chunk_index' => $chunk->metadata->chunkIndex,
                'content' => $chunk->content,
                'embedding' => $chunk->embedding,
                'metadata' => [
                    'title' => $result->metadata->title ?? basename($file),
                    'author' => $result->metadata->author ?? 'Unknown',
                ],
            ];
        }
    }
}

echo "Indexed " . count($documentIndex) . " chunks from " . count($files) . " documents\n\n";

function semanticSearch(array $index, array $queryEmbedding, int $topK = 5): array
{
    $results = [];

    foreach ($index as $item) {
        $similarity = cosineSimilarity($queryEmbedding, $item['embedding']);
        $results[] = array_merge($item, ['similarity' => $similarity]);
    }

    usort($results, fn($a, $b) => $b['similarity'] <=> $a['similarity']);

    return array_slice($results, 0, $topK);
}

function cosineSimilarity(array $a, array $b): float
{
    $dotProduct = $magnitudeA = $magnitudeB = 0.0;

    for ($i = 0; $i < count($a); $i++) {
        $dotProduct += $a[$i] * $b[$i];
        $magnitudeA += $a[$i] * $a[$i];
        $magnitudeB += $b[$i] * $b[$i];
    }

    return $dotProduct / (sqrt($magnitudeA) * sqrt($magnitudeB));
}

function getQueryEmbedding(Kreuzberg $kreuzberg, string $query): ?array
{
    $tempFile = tempnam(sys_get_temp_dir(), 'query_');
    file_put_contents($tempFile, $query);

    try {
        $result = $kreuzberg->extractFile($tempFile);
        $chunk = ($result->chunks ?? [])[0] ?? null;
        return $chunk?->embedding;
    } finally {
        unlink($tempFile);
    }
}

$queries = [
    "What are the key features of the product?",
    "How do I install and configure the system?",
    "What are the pricing options?",
    "How does authentication work?",
    "What are the performance benchmarks?",
];

foreach ($queries as $query) {
    echo "Query: \"$query\"\n";
    echo str_repeat('=', 60) . "\n";

    $queryEmbedding = getQueryEmbedding($kreuzberg, $query);

    if ($queryEmbedding) {
        $results = semanticSearch($documentIndex, $queryEmbedding, 3);

        foreach ($results as $index => $result) {
            echo "\nResult " . ($index + 1) . " (similarity: " .
                number_format($result['similarity'], 4) . "):\n";
            echo "File: {$result['file']}\n";
            echo "Title: {$result['metadata']['title']}\n";
            echo "Content: " . substr($result['content'], 0, 200) . "...\n";
        }
    }

    echo "\n" . str_repeat('-', 60) . "\n\n";
}

function buildRAGContext(array $searchResults, int $maxTokens = 2000): string
{
    $context = "Relevant context:\n\n";
    $currentTokens = 0;

    foreach ($searchResults as $result) {
        $tokens = strlen($result['content']) / 4; 

        if ($currentTokens + $tokens > $maxTokens) {
            break;
        }

        $context .= "From {$result['file']}:\n";
        $context .= $result['content'] . "\n\n";
        $currentTokens += $tokens;
    }

    return $context;
}

$userQuestion = "How do I optimize performance?";
$queryEmbedding = getQueryEmbedding($kreuzberg, $userQuestion);

if ($queryEmbedding) {
    $results = semanticSearch($documentIndex, $queryEmbedding, 5);
    $context = buildRAGContext($results);

    echo "RAG Context for: \"$userQuestion\"\n";
    echo str_repeat('=', 60) . "\n";
    echo $context;
    echo "\nContext ready for LLM prompt!\n";
}

file_put_contents(
    'document_index.json',
    json_encode($documentIndex, JSON_PRETTY_PRINT)
);
echo "\nSaved document index to: document_index.json\n";

function multiQuerySearch(array $index, array $queries, Kreuzberg $kreuzberg): array
{
    $allResults = [];

    foreach ($queries as $query) {
        $queryEmbedding = getQueryEmbedding($kreuzberg, $query);
        if ($queryEmbedding) {
            $results = semanticSearch($index, $queryEmbedding, 10);
            $allResults = array_merge($allResults, $results);
        }
    }

    $grouped = [];
    foreach ($allResults as $result) {
        $key = $result['file'] . '_' . $result['chunk_index'];
        if (!isset($grouped[$key])) {
            $grouped[$key] = [
                'result' => $result,
                'similarities' => [],
            ];
        }
        $grouped[$key]['similarities'][] = $result['similarity'];
    }

    $final = [];
    foreach ($grouped as $data) {
        $avgSimilarity = array_sum($data['similarities']) / count($data['similarities']);
        $final[] = array_merge($data['result'], ['avg_similarity' => $avgSimilarity]);
    }

    usort($final, fn($a, $b) => $b['avg_similarity'] <=> $a['avg_similarity']);

    return array_slice($final, 0, 5);
}

$relatedQueries = [
    "system requirements",
    "installation steps",
    "getting started guide",
];

echo "\nMulti-query search results:\n";
echo str_repeat('=', 60) . "\n";

$results = multiQuerySearch($documentIndex, $relatedQueries, $kreuzberg);

foreach ($results as $index => $result) {
    echo "\n" . ($index + 1) . ". {$result['file']}\n";
    echo "   Average similarity: " . number_format($result['avg_similarity'], 4) . "\n";
    echo "   " . substr($result['content'], 0, 150) . "...\n";
}
```
