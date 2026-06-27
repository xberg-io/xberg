```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;

$config = new ExtractionConfig();
$config->setChunking(new ChunkingConfig());
$result = Xberg::extractSync('document.pdf', null, $config);

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
