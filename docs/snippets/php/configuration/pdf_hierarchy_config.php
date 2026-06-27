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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Hierarchy detection enabled\n";
echo "Content length: " . strlen($result->getContent()) . " characters\n";

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('complex_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('simple_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Fast extraction without hierarchy detection\n";
```
