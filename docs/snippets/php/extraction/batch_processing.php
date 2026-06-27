```php title="batch_processing.php"
<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\ExtractInput;
use Xberg\ExtractionConfig;
use Xberg\Xberg;

$inputs = [
    ExtractInput::uri('document1.pdf'),
    ExtractInput::uri('document2.docx'),
    ExtractInput::bytes(file_get_contents('note.txt') ?: '', 'text/plain', 'note.txt'),
];

$config = new ExtractionConfig(
    extractTables: true,
    extractImages: false,
);

$output = Xberg::extractBatch($inputs, $config);

echo "Processed {$output->summary->results} documents\n";

foreach ($output->results as $result) {
    echo "Content: " . strlen($result->content) . " chars\n";
    echo "Tables: " . count($result->tables) . "\n";
    echo "MIME: {$result->mimeType}\n\n";
}
```
