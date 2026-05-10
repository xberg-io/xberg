```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\OcrConfig;
use Kreuzberg\ChunkingConfig;
use Kreuzberg\LanguageDetectionConfig;
use Kreuzberg\TokenReductionOptions;
use Kreuzberg\PostProcessorConfig;
use Kreuzberg\EmbeddingConfig;

// Advanced configuration combining multiple features
$config = new ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    ),
    chunking: new ChunkingConfig(
        maxCharacters: 1000,
        overlap: 200
    ),
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.8,
        detectMultiple: false
    ),
    tokenReduction: new TokenReductionOptions(
        mode: 'moderate',
        preserveImportantWords: true
    ),
    postprocessor: new PostProcessorConfig(
        enabled: true
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Content length: " . strlen($result->getContent()) . " characters\n";
if ($result->getDetectedLanguages()) {
    echo "Languages: " . implode(', ', $result->getDetectedLanguages()) . "\n";
}
?>
```
