```php
<?php

declare(strict_types=1);

/**
 * ExtractionConfig - Main Configuration
 *
 * The ExtractionConfig class is the primary configuration object that controls
 * all aspects of document extraction. It can be passed to the Kreuzberg constructor
 * or to individual extraction methods.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PdfConfig;

$config = new ExtractionConfig(
    extractImages: true,
    extractTables: true,
    preserveFormatting: false
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

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

$kreuzberg = new Kreuzberg($advancedConfig);
$result = $kreuzberg->extractFile('complex_document.pdf');

echo "Advanced extraction complete\n";
echo "Content format: " . ($advancedConfig->outputFormat ?? 'plain') . "\n";
echo "Formatting preserved: " . ($advancedConfig->preserveFormatting ? 'Yes' : 'No') . "\n";

$defaultConfig = new ExtractionConfig(extractTables: false);
$kreuzberg = new Kreuzberg($defaultConfig);

$result1 = $kreuzberg->extractFile('doc1.pdf');

$overrideConfig = new ExtractionConfig(extractTables: true);
$result2 = $kreuzberg->extractFile('doc2.pdf', config: $overrideConfig);

echo "\nDoc1 tables: " . count($result1->tables) . "\n";
echo "Doc2 tables: " . count($result2->tables) . "\n";
```
