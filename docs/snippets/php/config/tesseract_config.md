```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\OcrConfig;
use Kreuzberg\TesseractConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+deu',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            oem: 3
        )
    )
);

$result = Kreuzberg::extractFileSync('scanned.pdf', null, $config);

echo "OCR text: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
