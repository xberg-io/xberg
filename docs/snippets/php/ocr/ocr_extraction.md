```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

// Basic OCR extraction with Tesseract
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('scanned_document.pdf');

echo "Extracted Text:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

echo "Extraction Metadata:\n";
echo "Page count: " . ($result->metadata->pageCount ?? 'unknown') . "\n";
echo "Characters: " . strlen($result->content) . "\n";
echo "Tables found: " . count($result->tables) . "\n";

// Extract from image
if (file_exists('scanned_image.png')) {
    $imageResult = $xberg->extract('scanned_image.png');
    echo "\nImage OCR Results:\n";
    echo $imageResult->content . "\n";
}
?>
```
