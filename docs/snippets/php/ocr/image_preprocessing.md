```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;
use Xberg\Config\ImagePreprocessingConfig;

// Enhance OCR accuracy with image preprocessing
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,
            autoRotate: true,
            deskew: true,
            denoise: true,
            contrastEnhance: true,
            binarizationMethod: 'otsu',
            invertColors: false
        )
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Preprocessed OCR Results:\n";
echo "Characters extracted: " . strlen($result->getContent()) . "\n";
echo "Preview: " . substr($result->getContent(), 0, 300) . "...\n";
?>
```
