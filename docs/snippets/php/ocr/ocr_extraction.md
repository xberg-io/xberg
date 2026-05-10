```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

// Basic OCR extraction with Tesseract
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "Extracted Text:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

echo "Extraction Metadata:\n";
echo "Page count: " . ($result->metadata->pageCount ?? 'unknown') . "\n";
echo "Characters: " . strlen($result->content) . "\n";
echo "Tables found: " . count($result->tables) . "\n";

// Extract from image
if (file_exists('scanned_image.png')) {
    $imageResult = $kreuzberg->extractFile('scanned_image.png');
    echo "\nImage OCR Results:\n";
    echo $imageResult->content . "\n";
}
?>
```
