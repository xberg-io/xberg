```php title="PHP"
<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\Config\PdfConfig;

/**
 * PDF configuration with hierarchy detection
 */
$config = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        extractMetadata: true,
        passwords: ['password1', 'password2'],
        hierarchy: [
            'enabled' => true,
            'k_clusters' => 6,
            'include_bbox' => true,
            'ocr_coverage_threshold' => 0.5
        ]
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);
$result = $resultOutput->results[0];

echo "Content length: " . strlen($result->getContent()) . " characters\n";
echo "Metadata: " . implode(', ', array_keys((array) ($result->metadata?->pdf ?? []))) . "\n";
```
