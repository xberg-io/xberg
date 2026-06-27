```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;

$config = new ExtractionConfig(
    enableQualityProcessing: true,
    useCache: true
);

$result = Xberg::extractSync('document.pdf', null, $config);

if ($result->getQualityScore() !== null) {
    echo "Quality score: " . $result->getQualityScore() . "\n";
}

if ($result->getProcessingTime() !== null) {
    echo "Processing time: " . $result->getProcessingTime() . "ms\n";
}
?>
```
