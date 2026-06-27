```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ImageExtractionConfig;

$config = new ExtractionConfig(
    images: new ImageExtractionConfig(
        extractImages: true,
        targetDpi: 300,
        maxImageDimension: 4096,
        autoAdjustDpi: true,
        minDpi: 150,
        maxDpi: 600
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Extracted images: " . count($result->getImages()) . "\n";
?>
```
