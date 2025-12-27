```php
<?php

declare(strict_types=1);

/**
 * Image Extraction Configuration
 *
 * This example demonstrates how to configure image extraction from documents,
 * including size filtering and OCR on extracted images.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\OcrConfig;

echo "Example 1: Basic Image Extraction\n";
echo "=================================\n";

$config1 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true
    )
);

$kreuzberg = new Kreuzberg($config1);
$result = $kreuzberg->extractFile('presentation.pptx');

if ($result->images !== null) {
    echo "Total images extracted: " . count($result->images) . "\n";
    foreach ($result->images as $i => $image) {
        echo "\nImage {$i}:\n";
        echo "- Format: {$image->format}\n";
        echo "- Size: {$image->width}x{$image->height} pixels\n";
        echo "- Page: {$image->pageNumber}\n";
        echo "- Data size: " . strlen($image->data) . " bytes\n";
    }
}

echo "\n\n";

echo "Example 2: Image Extraction with Size Filter\n";
echo "============================================\n";

$config2 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 200,    
        minHeight: 200    
    )
);

$result2 = (new Kreuzberg($config2))->extractFile('document.pdf');

echo "Filtering images smaller than 200x200 pixels\n";
if ($result2->images !== null) {
    echo "Filtered images: " . count($result2->images) . "\n";
}

echo "\n\n";

echo "Example 3: Extract Only Large Images\n";
echo "====================================\n";

$config3 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 800,    
        minHeight: 600
    )
);

echo "Configured to extract images >= 800x600 pixels\n";
echo "Good for: Photos, large diagrams, full-page scans\n\n";

echo "Example 4: Extract All Images (Including Thumbnails)\n";
echo "===================================================\n";

$config4 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 50,     
        minHeight: 50
    )
);

echo "Configured to extract images >= 50x50 pixels\n";
echo "Good for: Extracting all images including icons and thumbnails\n\n";

echo "Example 5: Image Extraction with OCR\n";
echo "====================================\n";

$config5 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        performOcr: true,  
        minWidth: 100,
        minHeight: 100
    ),
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$result5 = (new Kreuzberg($config5))->extractFile('document_with_images.pdf');

if ($result5->images !== null) {
    echo "Extracted " . count($result5->images) . " images with OCR:\n\n";

    foreach ($result5->images as $i => $image) {
        echo "Image {$i} (Page {$image->pageNumber}):\n";
        echo "- Size: {$image->width}x{$image->height}\n";

        if ($image->ocrResult !== null) {
            echo "- OCR Text: " . substr($image->ocrResult->content, 0, 100) . "...\n";
            echo "- OCR Text Length: " . strlen($image->ocrResult->content) . " characters\n";
        }
        echo "\n";
    }
}

echo "\n\n";

echo "Example 6: Extract and Save Images to Disk\n";
echo "=========================================\n";

$config6 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 200,
        minHeight: 200
    )
);

$result6 = (new Kreuzberg($config6))->extractFile('presentation.pptx');

if ($result6->images !== null) {
    $outputDir = 'extracted_images';
    if (!is_dir($outputDir)) {
        mkdir($outputDir, 0755, true);
    }

    foreach ($result6->images as $i => $image) {
        $filename = "{$outputDir}/image_{$i}_page_{$image->pageNumber}.{$image->format}";

        $imageData = base64_decode($image->data);
        file_put_contents($filename, $imageData);

        echo "Saved: {$filename} ({$image->width}x{$image->height})\n";
    }
}

echo "\n\n";

echo "Example 7: File Type-Specific Image Extraction\n";
echo "==============================================\n";

$pdfConfig = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 300,
        minHeight: 300,
        performOcr: false  
    )
);

$pptxConfig = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 100,     
        minHeight: 100,
        performOcr: false
    )
);

$imageConfig = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        performOcr: true,  
        minWidth: 50,
        minHeight: 50
    ),
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

echo "PDF Configuration:\n";
echo "- Min size: 300x300 (larger images only)\n";
echo "- OCR: Disabled (PDFs have embedded text)\n\n";

echo "PowerPoint Configuration:\n";
echo "- Min size: 100x100 (include icons/logos)\n";
echo "- OCR: Disabled\n\n";

echo "Image File Configuration:\n";
echo "- Min size: 50x50 (all images)\n";
echo "- OCR: Enabled\n\n";

echo "Example 8: Complete Image Processing Pipeline\n";
echo "=============================================\n";

$config8 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        performOcr: true,
        minWidth: 200,
        minHeight: 200
    ),
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$result8 = (new Kreuzberg($config8))->extractFile('mixed_content.pdf');

if ($result8->images !== null) {
    echo "Extracted images: " . count($result8->images) . "\n\n";

    foreach ($result8->images as $i => $image) {
        echo "Processing Image {$i}:\n";

        $isValid = $image->width >= 200 && $image->height >= 200;
        echo "- Valid size: " . ($isValid ? 'Yes' : 'No') . "\n";

        $filename = "image_{$i}.{$image->format}";
        file_put_contents($filename, base64_decode($image->data));
        echo "- Saved: {$filename}\n";

        if ($image->ocrResult !== null) {
            $ocrText = trim($image->ocrResult->content);
            if (!empty($ocrText)) {
                echo "- OCR text available: " . strlen($ocrText) . " characters\n";
                file_put_contents("image_{$i}_ocr.txt", $ocrText);
            }
        }

        $metadata = [
            'format' => $image->format,
            'width' => $image->width,
            'height' => $image->height,
            'page' => $image->pageNumber,
            'aspect_ratio' => round($image->width / $image->height, 2),
        ];
        file_put_contents("image_{$i}_metadata.json", json_encode($metadata, JSON_PRETTY_PRINT));

        echo "- Metadata saved\n\n";
    }
}

echo "\nImage Extraction Configuration Parameters:\n";
echo "==========================================\n";
echo "- extractImages: Enable image extraction (default: false)\n";
echo "- performOcr: Run OCR on extracted images (default: false)\n";
echo "- minWidth: Minimum image width in pixels (default: 100)\n";
echo "- minHeight: Minimum image height in pixels (default: 100)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "- Set minWidth/minHeight to filter out unwanted small images\n";
echo "- Use 200x200 as a good default for meaningful images\n";
echo "- Use 800x600+ for large photos and diagrams only\n";
echo "- Use 50x50 to include all images including icons\n";
echo "- Enable performOcr only when images contain text\n";
echo "- Combine with OCR config for multilingual text in images\n";
echo "- Save images to disk for further processing\n";

echo "\n\nCommon Use Cases:\n";
echo "=================\n";
echo "1. Extract photos from reports: minWidth=800, minHeight=600\n";
echo "2. Extract all graphics: minWidth=100, minHeight=100\n";
echo "3. OCR on images: performOcr=true + OcrConfig\n";
echo "4. Extract logos/icons: minWidth=50, minHeight=50\n";
```
