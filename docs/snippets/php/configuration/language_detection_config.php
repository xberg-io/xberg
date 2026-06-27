```php title="language_detection_config.php"
<?php

declare(strict_types=1);

/**
 * LanguageDetectionConfig - Language Detection
 *
 * Automatically detect the languages present in a document.
 * Useful for multilingual documents and routing to appropriate OCR languages.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\LanguageDetectionConfig;

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('multilingual.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Detected languages:\n";
foreach ($result->detectedLanguages ?? [] as $lang) {
    echo "  - $lang\n";
}
echo "\n";

$advancedConfig = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        maxLanguages: 3,           
        confidenceThreshold: 0.8   
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

if (!empty($result->detectedLanguages)) {
    echo "High-confidence languages detected:\n";
    echo implode(', ', $result->detectedLanguages) . "\n\n";
} else {
    echo "No languages detected with sufficient confidence\n\n";
}

use Xberg\Config\OcrConfig;

$detectConfig = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(enabled: true)
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

if (!empty($result->detectedLanguages)) {
    $primaryLanguage = $result->detectedLanguages[0];
    echo "Primary language detected: $primaryLanguage\n";
    echo "Re-processing with OCR optimized for $primaryLanguage...\n";

    $ocrConfig = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: $primaryLanguage
        )
    );

    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
    echo "OCR extraction complete\n";
}

$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
$languageMap = [];

foreach ($files as $file) {
    if (!file_exists($file)) continue;

    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
    $lang = $result->detectedLanguages[0] ?? 'unknown';

    if (!isset($languageMap[$lang])) {
        $languageMap[$lang] = [];
    }
    $languageMap[$lang][] = $file;
}

echo "\nDocuments grouped by language:\n";
foreach ($languageMap as $lang => $docs) {
    echo "$lang: " . implode(', ', $docs) . "\n";
}
```
