```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'paddle-ocr',
        language: 'en'
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

if ($result->ocrElements !== null) {
    foreach ($result->ocrElements as $element) {
        echo "Text: {$element->text}\n";
        echo "Confidence: " . number_format($element->confidence->recognition, 2) . "\n";
        echo "Geometry: " . json_encode($element->geometry) . "\n";
        if ($element->rotation !== null) {
            echo "Rotation: {$element->rotation->angle}°\n";
        }
        echo "\n";
    }
}
```
