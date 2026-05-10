```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$result = Kreuzberg::extractFileSync('scanned.pdf', null, $config);

echo "Content length: " . strlen($result->getContent()) . " characters\n";
echo "Tables detected: " . count($result->getTables()) . "\n";
?>
```
