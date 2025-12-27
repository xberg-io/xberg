```php
<?php

declare(strict_types=1);

/**
 * Text Chunking Configuration
 *
 * This example demonstrates how to configure text chunking for RAG (Retrieval-Augmented Generation)
 * applications. Chunking splits long documents into smaller, semantically meaningful segments.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;

echo "Example 1: Basic Chunking\n";
echo "=========================\n";

$config1 = new ExtractionConfig(
    chunking: new ChunkingConfig()
);

$kreuzberg = new Kreuzberg($config1);
$result = $kreuzberg->extractFile('long_document.pdf');

if ($result->chunks !== null) {
    echo "Total chunks: " . count($result->chunks) . "\n";
    foreach ($result->chunks as $i => $chunk) {
        echo "\nChunk {$i}:\n";
        echo "- Text length: {$chunk->metadata->charCount} characters\n";
        echo "- Byte range: {$chunk->metadata->byteStart}-{$chunk->metadata->byteEnd}\n";
        if ($chunk->metadata->firstPage !== null) {
            echo "- Pages: {$chunk->metadata->firstPage}-{$chunk->metadata->lastPage}\n";
        }
    }
}

echo "\n\n";

echo "Example 2: Custom Chunk Size (Small chunks for fine-grained retrieval)\n";
echo "======================================================================\n";

$config2 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 256,      
        chunkOverlap: 25,       
        respectSentences: true, 
        respectParagraphs: false
    )
);

$result2 = (new Kreuzberg($config2))->extractFile('document.pdf');
echo "Chunks created: " . (isset($result2->chunks) ? count($result2->chunks) : 0) . "\n\n";

echo "Example 3: Large Chunks (More context per chunk)\n";
echo "================================================\n";

$config3 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 2000,      
        chunkOverlap: 200,       
        respectSentences: true,  
        respectParagraphs: true  
    )
);

$result3 = (new Kreuzberg($config3))->extractFile('document.pdf');
echo "Chunks created: " . (isset($result3->chunks) ? count($result3->chunks) : 0) . "\n\n";

echo "Example 4: RAG-Optimized Configuration\n";
echo "=====================================\n";

$config4 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,       
        chunkOverlap: 50,        
        respectSentences: true,  
        respectParagraphs: false 
    )
);

$result4 = (new Kreuzberg($config4))->extractFile('document.pdf');

if ($result4->chunks !== null) {
    echo "Total chunks: " . count($result4->chunks) . "\n";

    $chunkSizes = array_map(fn($chunk) => $chunk->metadata->charCount, $result4->chunks);
    echo "Average chunk size: " . round(array_sum($chunkSizes) / count($chunkSizes)) . " characters\n";
    echo "Min chunk size: " . min($chunkSizes) . " characters\n";
    echo "Max chunk size: " . max($chunkSizes) . " characters\n";
}

echo "\n\n";

echo "Example 5: Processing Chunks for Vector Database\n";
echo "================================================\n";

$config5 = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true
    )
);

$result5 = (new Kreuzberg($config5))->extractFile('document.pdf');

if ($result5->chunks !== null) {
    foreach ($result5->chunks as $i => $chunk) {
        $documentId = "doc_123";
        $chunkData = [
            'document_id' => $documentId,
            'chunk_index' => $i,
            'text' => $chunk->text,
            'char_count' => $chunk->metadata->charCount,
            'byte_start' => $chunk->metadata->byteStart,
            'byte_end' => $chunk->metadata->byteEnd,
            'page_range' => $chunk->metadata->firstPage !== null
                ? "{$chunk->metadata->firstPage}-{$chunk->metadata->lastPage}"
                : null,
        ];


        echo "Prepared chunk {$i} for database insertion\n";
    }
}

echo "\n\nChunking Configuration Parameters:\n";
echo "==================================\n";
echo "- maxChunkSize: Maximum number of characters per chunk\n";
echo "- chunkOverlap: Number of overlapping characters between chunks\n";
echo "- respectSentences: Split at sentence boundaries when possible\n";
echo "- respectParagraphs: Split at paragraph boundaries when possible\n";
echo "\nBest Practices:\n";
echo "- Use 256-512 chars for fine-grained retrieval\n";
echo "- Use 1000-2000 chars for more context\n";
echo "- Set overlap to ~10% of chunk size\n";
echo "- Enable respectSentences for better coherence\n";
echo "- Enable respectParagraphs for topic coherence\n";
```
