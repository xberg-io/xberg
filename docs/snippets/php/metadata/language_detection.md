```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\LanguageDetectionConfig;

// Configure language detection with confidence threshold
$langConfig = new LanguageDetectionConfig(
    enabled: true,
    minConfidence: 0.7,
    detectMultiple: false
);

$config = new ExtractionConfig();
$config->language_detection = $langConfig;

$result = Xberg::extract_sync("document.pdf", null, $config);

// Access detected languages
if (!empty($result->languages)) {
    foreach ($result->languages as $lang) {
        echo "Detected language: " . $lang->code . "\n";
        if ($lang->confidence !== null) {
            echo "Confidence: " . $lang->confidence . "\n";
        }
    }
}
?>
```
