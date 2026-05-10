```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class MyEmbedder implements EmbeddingBackend {
    public function name(): string {
        return "my-embedder";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Initialize the embedding model
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function dimensions(): int {
        return 768;
    }

    public function embed(array $texts): array {
        // Delegate to your already-loaded host model
        // Return array of embedding vectors
        $embeddings = [];
        foreach ($texts as $text) {
            $embeddings[] = array_fill(0, 768, 0.0);
        }
        return $embeddings;
    }
}

// Register the embedding backend at startup
$embedder = new MyEmbedder();
Kreuzberg::registerEmbeddingBackend($embedder);

// Use the registered backend in an EmbeddingConfig
$config = new EmbeddingConfig();
$config->model = "my-embedder";
$config->maxEmbedDurationSecs = 30;

$vectors = Kreuzberg::embedTexts(
    ["Hello, world!", "Second text"],
    $config
);

echo "Generated " . count($vectors) . " embeddings\n";
```
