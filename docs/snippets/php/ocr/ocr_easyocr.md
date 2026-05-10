```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

// Extract text using EasyOCR backend
// EasyOCR supports 90+ languages with multi-language detection
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'easyocr',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "EasyOCR Results:\n";
echo $result->content . "\n";

// Multi-language detection
$multiLangConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'easyocr',
        language: 'eng,fra,deu'  // English, French, German
    )
);

$kreuzberg = new Kreuzberg($multiLangConfig);
$result = $kreuzberg->extractFile('multilingual_document.pdf');

echo "\nMulti-language extraction:\n";
echo $result->content . "\n";
?>
```
