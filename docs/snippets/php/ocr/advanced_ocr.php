```php
<?php

declare(strict_types=1);

/**
 * Advanced OCR Configuration
 *
 * Fine-tune OCR performance with Tesseract configuration, image preprocessing,
 * and page segmentation modes.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,  
            enableTableDetection: true
        )
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('financial_report_scan.pdf');

echo "OCR with Table Detection:\n";
echo str_repeat('=', 60) . "\n";
echo "Tables found: " . count($result->tables) . "\n\n";

foreach ($result->tables as $index => $table) {
    echo "Table " . ($index + 1) . ":\n";
    echo $table->markdown . "\n\n";
}

$invoiceConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            tesseditCharWhitelist: '0123456789.,€$£¥-/'  
        )
    )
);

$kreuzberg = new Kreuzberg($invoiceConfig);
$result = $kreuzberg->extractFile('invoice_scan.pdf');

echo "Invoice OCR (numbers only):\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$preprocessedConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,      
            denoise: true,       
            sharpen: true,       
            autoRotate: true,    
            deskew: true        
        ),
        tesseractConfig: new TesseractConfig(
            psm: 3  
        )
    )
);

$kreuzberg = new Kreuzberg($preprocessedConfig);
$result = $kreuzberg->extractFile('poor_quality_scan.pdf');

echo "OCR with Image Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Extracted " . strlen($result->content) . " characters\n";
echo "Preview: " . substr($result->content, 0, 200) . "...\n\n";

$psmModes = [
    0 => 'Orientation and script detection (OSD) only',
    1 => 'Automatic page segmentation with OSD',
    3 => 'Fully automatic page segmentation (default)',
    4 => 'Assume a single column of text',
    5 => 'Assume a single uniform block of vertically aligned text',
    6 => 'Assume a single uniform block of text',
    7 => 'Treat the image as a single text line',
    8 => 'Treat the image as a single word',
    9 => 'Treat the image as a single word in a circle',
    10 => 'Treat the image as a single character',
    11 => 'Sparse text - find as much text as possible',
    13 => 'Raw line - treat as a single text line',
];

$testFile = 'various_layouts.pdf';
if (file_exists($testFile)) {
    echo "Testing different PSM modes:\n";
    echo str_repeat('=', 60) . "\n";

    foreach ([3, 4, 6, 11] as $psm) {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
                tesseractConfig: new TesseractConfig(psm: $psm)
            )
        );

        $kreuzberg = new Kreuzberg($config);
        $start = microtime(true);
        $result = $kreuzberg->extractFile($testFile);
        $elapsed = microtime(true) - $start;

        echo "PSM $psm - {$psmModes[$psm]}:\n";
        echo "  Time: " . number_format($elapsed, 3) . "s\n";
        echo "  Characters: " . strlen($result->content) . "\n";
        echo "  Preview: " . substr($result->content, 0, 80) . "...\n\n";
    }
}

$singleColumnConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 4  
        ),
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true
        )
    )
);

$kreuzberg = new Kreuzberg($singleColumnConfig);
$result = $kreuzberg->extractFile('book_scan.pdf');

echo "Single-column OCR:\n";
echo $result->content . "\n\n";

$sparseConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 11  
        ),
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true,
            sharpen: true
        )
    )
);

$kreuzberg = new Kreuzberg($sparseConfig);
$result = $kreuzberg->extractFile('receipt.jpg');

echo "Sparse text OCR (receipt):\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$highAccuracyConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 3,
            enableTableDetection: true
        ),
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 400,      
            denoise: true,
            sharpen: true,
            autoRotate: true,
            deskew: true,
            removeBackground: true
        )
    )
);

$kreuzberg = new Kreuzberg($highAccuracyConfig);
$result = $kreuzberg->extractFile('legal_document_scan.pdf');

echo "High-accuracy OCR:\n";
echo "Characters: " . strlen($result->content) . "\n";
echo "Tables: " . count($result->tables) . "\n";
```
