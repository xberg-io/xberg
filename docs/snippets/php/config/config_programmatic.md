```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\OcrConfig;
use Kreuzberg\ChunkingConfig;
use Kreuzberg\TesseractConfig;

$config = new ExtractionConfig(
    useCache: true,
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+deu',
        tesseractConfig: new TesseractConfig(psm: 6)
    ),
    chunking: new ChunkingConfig(
        maxCharacters: 1000,
        overlap: 200
    ),
    enableQualityProcessing: true
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Content length: " . strlen($result->getContent()) . " characters\n";
?>
```
