```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

// Force OCR on all pages, even those with native text
// Useful when native text extraction is unreliable or corrupted
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    ),
    // Force OCR on all pages instead of falling back to native text
    forceOcr: true
);

$xberg = new Xberg($config);
$result = $xberg->extract('mixed_scanned_document.pdf');

echo "Force OCR Results:\n";
echo "All pages processed with OCR\n";
echo "Characters extracted: " . strlen($result->content) . "\n";
echo "Content preview:\n";
echo substr($result->content, 0, 500) . "...\n";

// Without force OCR - uses native text when available
$nativeConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    ),
    forceOcr: false  // Default: use native text extraction when available
);

$xbergNative = new Xberg($nativeConfig);
$resultNative = $xbergNative->extract('mixed_scanned_document.pdf');

echo "\nNative Text Extraction (no force):\n";
echo "Characters extracted: " . strlen($resultNative->content) . "\n";
?>
```
