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
        detectMultiple: false
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Detected language: " . $result->getLanguage() . "\n";
echo "Confidence: " . $result->getLanguageConfidence() . "\n";
?>
```
