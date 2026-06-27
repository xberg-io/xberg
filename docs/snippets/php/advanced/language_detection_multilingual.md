```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\LanguageDetectionConfig;

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.8,
        detectMultiple: true
    )
);

$result = Xberg::extractSync('multilingual_document.pdf', null, $config);

echo "Detected languages: ";
$languages = $result->getDetectedLanguages();
if ($languages) {
    echo implode(", ", $languages) . "\n";
} else {
    echo "None\n";
}

echo "Primary language: " . $result->getLanguage() . "\n";
echo "Confidence: " . $result->getLanguageConfidence() . "\n";
?>
```
