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

$result = Xberg::extractSync('document.pdf', null, $config);

echo "Detected language: " . $result->getLanguage() . "\n";
echo "Confidence: " . $result->getLanguageConfidence() . "\n";
?>
```
