<?php

declare(strict_types=1);

/**
 * Text Chunking Example
 *
 * Demonstrates text chunking for RAG (Retrieval-Augmented Generation) applications.
 * Shows various chunking strategies and semantic chunking techniques.
 *
 * This example covers:
 * - Basic text chunking with overlap
 * - Sentence-aware chunking
 * - Paragraph-aware chunking
 * - Custom chunk sizes
 * - Accessing chunk metadata
 * - Chunking with page boundaries
 * - Combining chunking with embeddings
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_file;


echo "=== Example 1: Basic Text Chunking ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

    echo "Basic chunking results:\n";
    echo "  Total chunks: " . count($result->chunks ?? []) . "\n";
    echo "  Full content length: " . strlen($result->content) . " characters\n";

    if ($result->chunks !== null) {
        echo "\nFirst 3 chunks:\n";
        foreach (array_slice($result->chunks, 0, 3) as $chunk) {
            echo "\n  Chunk {$chunk->metadata->chunkIndex}:\n";
            echo "    Length: " . strlen($chunk->content) . " characters\n";
            echo "    Byte range: {$chunk->metadata->byteStart}-{$chunk->metadata->byteEnd}\n";
            echo "    Token count: " . ($chunk->metadata->tokenCount ?? 'N/A') . "\n";
            echo "    Total chunks: {$chunk->metadata->totalChunks}\n";
            echo "    Preview: " . substr($chunk->content, 0, 100) . "...\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Small Chunks for Fine-Grained Retrieval ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 256,
            chunkOverlap: 25,
            respectSentences: true,
            respectParagraphs: false,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    if ($result->chunks !== null) {
        echo "Small chunks (256 chars):\n";
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  Average chunk size: " . round(
            array_sum(array_map(
                static fn ($chunk) => strlen($chunk->content),
                $result->chunks
            )) / count($result->chunks)
        ) . " characters\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Large Chunks for Context ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 2048,
            chunkOverlap: 200,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/report.pdf');

    if ($result->chunks !== null) {
        echo "Large chunks (2048 chars):\n";
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  Average chunk size: " . round(
            array_sum(array_map(
                static fn ($chunk) => strlen($chunk->content),
                $result->chunks
            )) / count($result->chunks)
        ) . " characters\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Sentence-Aware Chunking ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: false,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

    echo "Sentence-aware chunking:\n";
    if ($result->chunks !== null) {
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "\nExample chunks:\n";

        foreach (array_slice($result->chunks, 0, 2) as $chunk) {
            echo "\n  Chunk {$chunk->metadata->chunkIndex}:\n";
            echo "    " . str_replace("\n", "\n    ", $chunk->content) . "\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: Paragraph-Aware Chunking ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 1024,
            chunkOverlap: 100,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    echo "Paragraph-aware chunking:\n";
    if ($result->chunks !== null) {
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  Note: Chunks split at paragraph boundaries when possible\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Chunk Overlap Analysis ===\n\n";

try {
    $overlapSizes = [0, 25, 50, 100];

    foreach ($overlapSizes as $overlap) {
        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(
                maxChunkSize: 512,
                chunkOverlap: $overlap,
                respectSentences: true,
                respectParagraphs: true,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

        if ($result->chunks !== null) {
            echo "Overlap: {$overlap} characters\n";
            echo "  Total chunks: " . count($result->chunks) . "\n";
            echo "  Average chunk size: " . round(
                array_sum(array_map(
                    static fn ($chunk) => strlen($chunk->content),
                    $result->chunks
                )) / count($result->chunks)
            ) . " characters\n\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Chunking with Page Information ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Chunks with page information:\n";
    if ($result->chunks !== null) {
        foreach (array_slice($result->chunks, 0, 5) as $chunk) {
            $pages = '';
            if ($chunk->metadata->firstPage !== null && $chunk->metadata->lastPage !== null) {
                if ($chunk->metadata->firstPage === $chunk->metadata->lastPage) {
                    $pages = "Page {$chunk->metadata->firstPage}";
                } else {
                    $pages = "Pages {$chunk->metadata->firstPage}-{$chunk->metadata->lastPage}";
                }
            }

            echo "\n  Chunk {$chunk->metadata->chunkIndex}";
            if ($pages !== '') {
                echo " ({$pages})";
            }
            echo ":\n";
            echo "    Length: " . strlen($chunk->content) . " characters\n";
            echo "    Preview: " . substr($chunk->content, 0, 80) . "...\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 8: Procedural API for Chunking ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $result = extract_file(
        __DIR__ . '/../sample-documents/document.pdf',
        config: $config
    );

    echo "Procedural API chunking:\n";
    if ($result->chunks !== null) {
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  First chunk length: " . strlen($result->chunks[0]->content) . " characters\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 9: Iterating and Processing Chunks ===\n\n";

try {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

    if ($result->chunks !== null) {
        echo "Processing chunks:\n\n";

        $keyword = 'example';
        $matchingChunks = array_filter(
            $result->chunks,
            static fn ($chunk) => stripos($chunk->content, $keyword) !== false
        );

        echo "Chunks containing '{$keyword}': " . count($matchingChunks) . "\n";

        $chunkLengths = array_map(
            static fn ($chunk) => strlen($chunk->content),
            $result->chunks
        );

        echo "\nChunk statistics:\n";
        echo "  Total chunks: " . count($result->chunks) . "\n";
        echo "  Minimum length: " . min($chunkLengths) . " characters\n";
        echo "  Maximum length: " . max($chunkLengths) . " characters\n";
        echo "  Average length: " . round(array_sum($chunkLengths) / count($chunkLengths)) . " characters\n";

        $chunksByPage = [];
        foreach ($result->chunks as $chunk) {
            if ($chunk->metadata->firstPage !== null) {
                $page = $chunk->metadata->firstPage;
                if (!isset($chunksByPage[$page])) {
                    $chunksByPage[$page] = [];
                }
                $chunksByPage[$page][] = $chunk;
            }
        }

        echo "\nChunks per page:\n";
        foreach ($chunksByPage as $page => $chunks) {
            echo "  Page {$page}: " . count($chunks) . " chunks\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 10: Comparing Chunking Strategies ===\n\n";

try {
    $filePath = __DIR__ . '/../sample-documents/document.pdf';

    $strategies = [
        'Small chunks' => new ChunkingConfig(
            maxChunkSize: 256,
            chunkOverlap: 25,
            respectSentences: true,
            respectParagraphs: false,
        ),
        'Medium chunks' => new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),
        'Large chunks' => new ChunkingConfig(
            maxChunkSize: 1024,
            chunkOverlap: 100,
            respectSentences: true,
            respectParagraphs: true,
        ),
        'No overlap' => new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 0,
            respectSentences: true,
            respectParagraphs: true,
        ),
    ];

    echo "Comparing chunking strategies:\n\n";

    foreach ($strategies as $name => $chunkingConfig) {
        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if ($result->chunks !== null) {
            $avgLength = round(
                array_sum(array_map(
                    static fn ($chunk) => strlen($chunk->content),
                    $result->chunks
                )) / count($result->chunks)
            );

            echo "{$name}:\n";
            echo "  Total chunks: " . count($result->chunks) . "\n";
            echo "  Average length: {$avgLength} characters\n";
            echo "  Coverage: " . round(
                (array_sum(array_map(
                    static fn ($chunk) => strlen($chunk->content),
                    $result->chunks
                )) / strlen($result->content)) * 100,
                1
            ) . "%\n\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "Done!\n";
