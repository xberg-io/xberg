```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\LanguageDetectionConfig;

// Configure multilingual language detection
$langConfig = new LanguageDetectionConfig(
    enabled: true,
    minConfidence: 0.6,
    detectMultiple: true
);

$config = new ExtractionConfig();
$config->language_detection = $langConfig;

$result = Xberg::extract_sync("multilingual_document.pdf", null, $config);

// Iterate through all detected languages
if (!empty($result->languages)) {
    echo "Detected " . count($result->languages) . " language(s):\n";

    foreach ($result->languages as $lang) {
        echo "Language: " . $lang->code . "\n";
        if ($lang->confidence !== null) {
            printf("  Confidence: %.1f%%\n", $lang->confidence * 100);
        }
        if ($lang->name !== null) {
            echo "  Name: " . $lang->name . "\n";
        }
    }
} else {
    echo "No languages detected\n";
}
?>
```
