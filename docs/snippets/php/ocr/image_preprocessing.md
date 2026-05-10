```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

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

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "Preprocessed OCR Results:\n";
echo "Characters extracted: " . strlen($result->content) . "\n";
echo "Preview: " . substr($result->content, 0, 300) . "...\n";
?>
```
