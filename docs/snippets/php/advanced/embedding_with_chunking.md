```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;
use Xberg\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 1024,
        overlap: 100,
        embedding: new EmbeddingConfig(
            normalize: true,
            batchSize: 32,
            showDownloadProgress: false
        )
    )
);

$result = Xberg::extractSync('document.pdf', null, $config);

if ($result->getChunks()) {
    foreach ($result->getChunks() as $chunk) {
        echo "Chunk content: " . substr($chunk->getContent(), 0, 100) . "...\n";

        $embedding = $chunk->getEmbedding();
        if ($embedding) {
            echo "Embedding dimension: " . count($embedding) . "\n";
            echo "First 5 values: ";
            echo implode(", ", array_slice($embedding, 0, 5));
            echo "\n";
        }
        echo "\n";
    }
}
?>
```
