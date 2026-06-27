```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;

$config = ExtractionConfig::default();
$config->setChunking(new ChunkingConfig());
$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);
$result = $resultOutput->results[0];

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
