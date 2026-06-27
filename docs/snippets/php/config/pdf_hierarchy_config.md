```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PdfConfig;
use Xberg\HierarchyConfig;

$config = new ExtractionConfig(
    pdfOptions: new PdfConfig(
        hierarchy: new HierarchyConfig(
            enabled: true,
            detectionThreshold: 0.75,
            ocrCoverageThreshold: 0.8,
            minLevel: 1,
            maxLevel: 5
        )
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Hierarchy levels: " . count($result->getHierarchy()) . "\n";
?>
```
