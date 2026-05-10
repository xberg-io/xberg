```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$config = new ExtractionConfig(
    enableQualityProcessing: true
);

$result = Kreuzberg::extractFileSync('scanned_document.pdf', null, $config);

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
