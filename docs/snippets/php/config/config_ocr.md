```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$result = Xberg::extractSync('scanned.pdf', null, $config);

echo "Content length: " . strlen($result->getContent()) . " characters\n";
echo "Tables detected: " . count($result->getTables()) . "\n";
?>
```
