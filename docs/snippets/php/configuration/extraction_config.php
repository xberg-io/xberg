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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('complex_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Advanced extraction complete\n";
echo "Content format: " . ($advancedConfig->outputFormat ?? 'plain') . "\n";
echo "Formatting preserved: " . ($advancedConfig->preserveFormatting ? 'Yes' : 'No') . "\n";

$defaultConfig = new ExtractionConfig(extractTables: false);

$result1 = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('doc1.pdf'), $config ?? \Xberg\ExtractionConfig::default())->results[0];

$overrideConfig = new ExtractionConfig(extractTables: true);
$result2 = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('doc2.pdf'), $overrideConfig)->results[0];

echo "\nDoc1 tables: " . count($result1->tables) . "\n";
echo "Doc2 tables: " . count($result2->tables) . "\n";
```
