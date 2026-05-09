```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\ChunkingConfig;

$config = new ExtractionConfig();
$config->setChunking(new ChunkingConfig());
$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Total content length: " . strlen($result->getContent()) . "\n";

if ($result->getChunks() !== null) {
    foreach ($result->getChunks() as $chunk) {
        echo "Chunk: " . $chunk->getContent() . "\n";
    }
}

foreach ($result->getTables() as $table) {
    echo "Table with " . count($table->getRows()) . " rows\n";
}
```
