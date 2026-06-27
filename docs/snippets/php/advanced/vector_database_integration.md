```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;
use Xberg\EmbeddingConfig;

class VectorRecord {
    public function __construct(
        public string $id,
        public string $content,
        public array $embedding,
        public array $metadata
    ) {}
}

function extractAndVectorize(
    string $documentPath,
    string $documentId
): array {
    $config = new ExtractionConfig(
        chunking: new ChunkingConfig(
            maxCharacters: 512,
            overlap: 50,
            embedding: new EmbeddingConfig(
                normalize: true,
                batchSize: 32
            )
        )
    );

    $result = Xberg::extractSync($documentPath, null, $config);

    $records = [];
    if ($result->getChunks()) {
        foreach ($result->getChunks() as $index => $chunk) {
            $embedding = $chunk->getEmbedding();
            if ($embedding) {
                $metadata = [
                    'document_id' => $documentId,
                    'chunk_index' => (string)$index,
                    'content_length' => (string)strlen($chunk->getContent()),
                ];

                $records[] = new VectorRecord(
                    id: "{$documentId}_chunk_{$index}",
                    content: $chunk->getContent(),
                    embedding: $embedding,
                    metadata: $metadata
                );
            }
        }
    }

    return $records;
}

// Usage
$records = extractAndVectorize('research_paper.pdf', 'doc_123');

foreach ($records as $record) {
    echo "Vector ID: " . $record->id . "\n";
    echo "Content length: " . strlen($record->content) . " characters\n";
    echo "Embedding dimension: " . count($record->embedding) . "\n";
    echo "---\n";
}
?>
```
