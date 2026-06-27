```php title="pdf_hierarchy_config.php"
<?php

declare(strict_types=1);

/**
 * PdfHierarchyConfig - Hierarchy Detection Configuration
 *
 * Configure PDF document structure analysis and hierarchy detection
 * using k-clustering for document organization recognition.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\PdfConfig;

// Hierarchy detection in PDF options array
$config = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        hierarchy: [
            'enabled' => true,
            'k_clusters' => 6,
            'include_bbox' => true,
            'ocr_coverage_threshold' => 0.8
        ]
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

echo "Hierarchy detection enabled\n";
echo "Content length: " . strlen($result->content) . " characters\n";

// Alternative: Custom hierarchy parameters for complex documents
$advancedConfig = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        hierarchy: [
            'enabled' => true,
            'k_clusters' => 12,           // More clusters for detailed hierarchy
            'include_bbox' => true,       // Include bounding box coordinates
            'ocr_coverage_threshold' => 0.7  // Higher OCR threshold
        ]
    )
);

$xberg = new Xberg($advancedConfig);
$result = $xberg->extract('complex_document.pdf');

echo "Advanced hierarchy detection completed\n";
echo "Detected structure preserved in output\n";

// Disabling hierarchy detection for speed
$fastConfig = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: false,
        hierarchy: [
            'enabled' => false
        ]
    )
);

$xberg = new Xberg($fastConfig);
$result = $xberg->extract('simple_document.pdf');

echo "Fast extraction without hierarchy detection\n";
```
