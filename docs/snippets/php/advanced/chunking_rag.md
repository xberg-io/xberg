```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;
use Xberg\EmbeddingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 500,
        overlap: 50,
        embedding: new EmbeddingConfig(
            normalize: true,
            batchSize: 32
        )
    )
);

$result = Xberg::extractSync('research_paper.pdf', null, $config);

if ($result->getChunks()) {
    foreach ($result->getChunks() as $chunk) {
        $metadata = $chunk->getMetadata();
        if ($metadata) {
            echo "Chunk " . ($metadata->getChunkIndex() + 1) . "/" . $metadata->getTotalChunks() . "\n";
            echo "Position: " . $metadata->getByteStart() . "-" . $metadata->getByteEnd() . "\n";
            echo "Content: " . substr($chunk->getContent(), 0, 100) . "...\n";

            if ($chunk->getEmbedding()) {
                echo "Embedding: " . count($chunk->getEmbedding()) . " dimensions\n";
            }
        }
        echo "\n";
    }
}
?>
```
