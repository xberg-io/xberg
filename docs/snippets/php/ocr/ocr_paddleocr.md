```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'paddle-ocr',
        language: 'en',
        // paddleOcrConfig: new PaddleOcrConfig(modelTier: 'server') // for max accuracy
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo $result->getContent() . "\n";
```
