```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\ChunkingConfig;
use Kreuzberg\EmbeddingConfig;

// Configure chunking with embedding generation for vector database
$chunkConfig = new ChunkingConfig(
    enableChunking: true,
    chunkSize: 512,
    chunkOverlap: 50,
    chunker: "semantic"
);

$embeddingConfig = new EmbeddingConfig(
    generateEmbeddings: true,
    modelName: "all-minilm-l6-v2"
);

$config = new ExtractionConfig();
$config->chunking = $chunkConfig;
$config->embeddings = $embeddingConfig;

$result = Kreuzberg::extract_file_sync("document.pdf", null, $config);

// Store chunks and embeddings for vector database
if ($result->chunks !== null) {
    foreach ($result->chunks as $chunk) {
        // Store in vector database with embedding
        $vectorRecord = [
            "text" => $chunk->text,
            "embedding" => $chunk->embedding ?? [],
            "metadata" => [
                "source" => "document.pdf",
                "page" => $chunk->page_number ?? null,
                "chunk_id" => $chunk->chunk_id ?? null,
            ]
        ];
        
        // Insert into vector DB (e.g., Pinecone, Weaviate, Milvus)
        // storeInVectorDB($vectorRecord);
        
        echo "Chunk: " . substr($chunk->text, 0, 50) . "...\n";
        echo "Embedding dimensions: " . count($chunk->embedding ?? []) . "\n";
    }
}
?>
```
