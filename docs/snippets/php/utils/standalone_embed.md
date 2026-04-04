```php
<?php
use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\EmbeddingModelType;

$kreuzberg = new Kreuzberg();

// Embed with default config (balanced preset)
$embeddings = $kreuzberg->embed(["Hello world", "How are you?"]);

// Embed with specific preset
$config = new EmbeddingConfig(model: EmbeddingModelType::preset("fast"));
$embeddings = $kreuzberg->embed(["Hello world"], $config);

// Each embedding is a float array
foreach ($embeddings as $i => $vector) {
    echo "Text $i: " . count($vector) . " dimensions\n";
}
```
