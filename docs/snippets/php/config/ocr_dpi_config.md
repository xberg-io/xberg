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

$result = Xberg::extractSync('document.pdf', null, $config);

echo "Extracted images: " . count($result->getImages()) . "\n";
?>
```
