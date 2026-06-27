```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;
use Xberg\ChunkingConfig;
use Xberg\TesseractConfig;

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

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Content length: " . strlen($result->getContent()) . " characters\n";
?>
```
