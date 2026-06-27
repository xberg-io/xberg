```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;
use Xberg\TesseractConfig;

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

$result = Xberg::extractSync('scanned.pdf', null, $config);

echo "OCR text: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
