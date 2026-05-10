```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\PdfConfig;
use Kreuzberg\HierarchyConfig;

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

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Hierarchy levels: " . count($result->getHierarchy()) . "\n";
?>
```
