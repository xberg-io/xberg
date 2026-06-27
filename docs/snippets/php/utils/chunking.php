```php title="chunking.php"
<?php

declare(strict_types=1);

/**
 * Text Chunking Configuration
 *
 * Configure document chunking for processing long texts into manageable pieces.
 * Useful for RAG systems, embedding generation, and token limit management.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\ChunkingConfig;
use Xberg\Config\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChars: 1500,
        maxOverlap: 200,
        embedding: new EmbeddingConfig(
            model: 'balanced'
        )
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

echo "Chunking Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Total chunks created: " . count($result->chunks ?? []) . "\n\n";

foreach ($result->chunks ?? [] as $index => $chunk) {
    echo "Chunk " . ($index + 1) . ":\n";
    echo "  Length: " . strlen($chunk->content) . " characters\n";
    echo "  Preview: " . substr($chunk->content, 0, 100) . "...\n";

    if ($chunk->embedding !== null) {
        echo "  Embedding dimensions: " . count($chunk->embedding) . "\n";
    }

    echo "\n";
}
```
