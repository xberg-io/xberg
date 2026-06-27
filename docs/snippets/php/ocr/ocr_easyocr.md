```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

// Extract text using EasyOCR backend
// EasyOCR supports 90+ languages with multi-language detection
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'easyocr',
        language: 'eng'
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

echo "EasyOCR Results:\n";
echo $result->content . "\n";

// Multi-language detection
$multiLangConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'easyocr',
        language: 'eng,fra,deu'  // English, French, German
    )
);

$xberg = new Xberg($multiLangConfig);
$result = $xberg->extract('multilingual_document.pdf');

echo "\nMulti-language extraction:\n";
echo $result->content . "\n";
?>
```
