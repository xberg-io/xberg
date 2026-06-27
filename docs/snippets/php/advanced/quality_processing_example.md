```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;

$config = new ExtractionConfig(
    enableQualityProcessing: true
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('scanned_document.pdf'), $config);

$result = $resultOutput->results[0];

if ($result->getQualityScore() !== null) {
    $score = $result->getQualityScore();
    if ($score < 0.5) {
        echo "Warning: Low quality extraction (" . round($score, 2) . ")\n";
    } else {
        echo "Quality score: " . round($score, 2) . "\n";
    }
} else {
    echo "Quality score not available.\n";
}

echo "Extracted text length: " . strlen($result->getContent()) . " characters\n";
?>
```
