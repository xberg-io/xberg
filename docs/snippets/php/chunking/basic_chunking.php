```php
<?php

declare(strict_types=1);

/**
 * Basic Text Chunking
 *
 * Split documents into smaller chunks for RAG (Retrieval Augmented Generation),
 * vector databases, and context-aware processing.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('long_document.pdf');

echo "Document Chunking Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Total chunks: " . count($result->chunks ?? []) . "\n";
echo "Total content length: " . strlen($result->content) . "\n\n";

foreach ($result->chunks ?? [] as $chunk) {
    echo "Chunk {$chunk->metadata->chunkIndex}:\n";
    echo str_repeat('-', 60) . "\n";
    echo "Length: " . strlen($chunk->content) . " chars\n";
    echo "Content: " . substr($chunk->content, 0, 100) . "...\n\n";
}

$sizes = [
    'Small (256)' => 256,   
    'Medium (512)' => 512,  
    'Large (1024)' => 1024, 
    'XLarge (2048)' => 2048, 
];

foreach ($sizes as $name => $size) {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: $size,
            chunkOverlap: (int)($size * 0.1)  
        )
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile('document.pdf');

    echo "$name chunks:\n";
    echo "  Total: " . count($result->chunks ?? []) . "\n";
    echo "  Avg size: " . number_format(
        array_sum(array_map(
            fn($c) => strlen($c->content),
            $result->chunks ?? []
        )) / count($result->chunks ?? [1])
    ) . " chars\n\n";
}

$sentenceConfig = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true,  
        respectParagraphs: false
    )
);

$kreuzberg = new Kreuzberg($sentenceConfig);
$result = $kreuzberg->extractFile('article.pdf');

echo "Sentence-respecting chunks:\n";
echo str_repeat('=', 60) . "\n";

foreach ($result->chunks ?? [] as $chunk) {
    $sentences = preg_match_all('/[.!?]+/', $chunk->content);
    echo "Chunk {$chunk->metadata->chunkIndex}: $sentences sentences\n";
    echo "  Starts with: " . substr($chunk->content, 0, 50) . "...\n";
    echo "  Ends with: ..." . substr($chunk->content, -50) . "\n\n";
}

$paragraphConfig = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 1000,
        chunkOverlap: 100,
        respectSentences: true,
        respectParagraphs: true  
    )
);

$kreuzberg = new Kreuzberg($paragraphConfig);
$result = $kreuzberg->extractFile('essay.pdf');

echo "Paragraph-respecting chunks:\n";
echo str_repeat('=', 60) . "\n";

foreach ($result->chunks ?? [] as $chunk) {
    $paragraphs = substr_count($chunk->content, "\n\n");
    echo "Chunk {$chunk->metadata->chunkIndex}: ~$paragraphs paragraphs\n";
    echo "  " . strlen($chunk->content) . " characters\n\n";
}

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('knowledge_base.pdf');

$chunksForDb = [];
foreach ($result->chunks ?? [] as $chunk) {
    $chunksForDb[] = [
        'id' => uniqid('chunk_', true),
        'document_id' => 'doc_' . md5($result->content),
        'chunk_index' => $chunk->metadata->chunkIndex,
        'content' => $chunk->content,
        'length' => strlen($chunk->content),
        'metadata' => [
            'source_file' => 'knowledge_base.pdf',
            'mime_type' => $result->mimeType,
            'created_at' => date('Y-m-d H:i:s'),
        ],
    ];
}

echo "Prepared " . count($chunksForDb) . " chunks for database:\n";
foreach (array_slice($chunksForDb, 0, 3) as $chunk) {
    echo "  ID: {$chunk['id']}\n";
    echo "  Index: {$chunk['chunk_index']}\n";
    echo "  Length: {$chunk['length']} chars\n\n";
}

file_put_contents(
    'chunks.json',
    json_encode($chunksForDb, JSON_PRETTY_PRINT)
);
echo "Saved chunks to: chunks.json\n";
```
