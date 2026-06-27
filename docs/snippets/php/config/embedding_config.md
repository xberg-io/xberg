```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;
use Xberg\EmbeddingConfig;

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

$result = Xberg::extractSync('document.pdf', null, $config);

echo "Chunks with embeddings: " . count($result->getChunks()) . "\n";
?>
```
