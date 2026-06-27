```php title="extraction_config.php"
<?php

declare(strict_types=1);

/**
 * ExtractionConfig - Main Configuration
 *
 * The ExtractionConfig class is the primary configuration object that controls
 * all aspects of document extraction. It can be passed to the Xberg constructor
 * or to individual extraction methods.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;
use Xberg\Config\PdfConfig;

$config = new ExtractionConfig(
    extractImages: true,
    extractTables: true,
    preserveFormatting: false
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

echo "Extracted with images: " . count($result->images ?? []) . "\n";
echo "Extracted with tables: " . count($result->tables) . "\n\n";

$advancedConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    ),
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 95
    ),
    extractImages: true,
    extractTables: true,
    preserveFormatting: true,
    outputFormat: 'markdown'
);

$xberg = new Xberg($advancedConfig);
$result = $xberg->extract('complex_document.pdf');

echo "Advanced extraction complete\n";
echo "Content format: " . ($advancedConfig->outputFormat ?? 'plain') . "\n";
echo "Formatting preserved: " . ($advancedConfig->preserveFormatting ? 'Yes' : 'No') . "\n";

$defaultConfig = new ExtractionConfig(extractTables: false);
$xberg = new Xberg($defaultConfig);

$result1 = $xberg->extract('doc1.pdf');

$overrideConfig = new ExtractionConfig(extractTables: true);
$result2 = $xberg->extract('doc2.pdf', config: $overrideConfig);

echo "\nDoc1 tables: " . count($result1->tables) . "\n";
echo "Doc2 tables: " . count($result2->tables) . "\n";
```
