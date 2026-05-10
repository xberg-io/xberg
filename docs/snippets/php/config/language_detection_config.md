```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\LanguageDetectionConfig;

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.8,
        detectMultiple: true
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Detected language: " . $result->getLanguage() . "\n";
echo "Confidence: " . $result->getLanguageConfidence() . "\n";
?>
```
