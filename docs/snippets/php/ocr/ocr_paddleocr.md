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

$xberg = new Xberg($config);
$result = $xberg->extract('scanned_document.pdf');

echo $result->content . "\n";
```
