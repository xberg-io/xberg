```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\ChunkingConfig;
use Kreuzberg\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 1000,
        overlap: 200,
        embedding: new EmbeddingConfig(
            model: 'balanced',
            batchSize: 16,
            normalize: true,
            showDownloadProgress: true
        )
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Chunks with embeddings: " . count($result->getChunks()) . "\n";
?>
```
