```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;
use Xberg\ChunkingConfig;
use Xberg\LanguageDetectionConfig;
use Xberg\TokenReductionOptions;
use Xberg\PostProcessorConfig;
use Xberg\EmbeddingConfig;

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

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Content length: " . strlen($result->getContent()) . " characters\n";
if ($result->getDetectedLanguages()) {
    echo "Languages: " . implode(', ', $result->getDetectedLanguages()) . "\n";
}
?>
```
