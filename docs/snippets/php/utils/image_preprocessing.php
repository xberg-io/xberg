```php
<?php

declare(strict_types=1);

/**
 * Image Preprocessing for OCR
 *
 * Configure image preprocessing settings to improve OCR accuracy on scanned documents.
 * Demonstrates various preprocessing techniques like denoising, deskewing, and contrast enhancement.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        tesseractConfig: new TesseractConfig(
            preprocessing: new ImagePreprocessingConfig(
                targetDpi: 300,
                denoise: true,
                deskew: true,
                contrastEnhance: true,
                binarizationMethod: 'otsu'
            )
        )
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned.pdf');

echo "OCR with Image Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Content extracted: " . strlen($result->content) . " characters\n";
echo "Preview: " . substr($result->content, 0, 100) . "...\n\n";

$advancedConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            preprocessing: new ImagePreprocessingConfig(
                targetDpi: 600,          
                denoise: true,           
                deskew: true,            
                contrastEnhance: true,   
                binarizationMethod: 'adaptive', 
                sharpen: true,           
                removeBackground: true   
            ),
            pageSegmentationMode: 3,
            engineMode: 3
        )
    )
);

$kreuzberg = new Kreuzberg($advancedConfig);
$result = $kreuzberg->extractFile('poor_quality_scan.pdf');

echo "Advanced Preprocessing Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Content length: " . strlen($result->content) . " characters\n";

if (isset($result->metadata)) {
    $qualityScore = $result->metadata['quality_score'] ?? null;
    $confidence = $result->metadata['ocr_confidence'] ?? null;

    if ($qualityScore !== null) {
        echo "Quality score: " . number_format($qualityScore, 2) . "\n";

        if ($qualityScore < 0.5) {
            echo "Warning: Low quality extraction detected\n";
            echo "Recommendations:\n";
            echo "  - Increase target DPI (current: 600)\n";
            echo "  - Try different binarization method\n";
            echo "  - Consider rescanning the original document\n";
        }
    }

    if ($confidence !== null) {
        echo "OCR confidence: " . number_format($confidence * 100, 1) . "%\n";
    }
}

echo "\n";

$preprocessingProfiles = [
    'basic' => new ImagePreprocessingConfig(
        targetDpi: 300,
        denoise: false,
        deskew: false,
        contrastEnhance: false
    ),
    'balanced' => new ImagePreprocessingConfig(
        targetDpi: 300,
        denoise: true,
        deskew: true,
        contrastEnhance: true,
        binarizationMethod: 'otsu'
    ),
    'aggressive' => new ImagePreprocessingConfig(
        targetDpi: 600,
        denoise: true,
        deskew: true,
        contrastEnhance: true,
        binarizationMethod: 'adaptive',
        sharpen: true,
        removeBackground: true
    ),
];

echo "Preprocessing Profile Comparison:\n";
echo str_repeat('=', 60) . "\n";

foreach ($preprocessingProfiles as $profileName => $preprocessing) {
    $profileConfig = new ExtractionConfig(
        ocr: new OcrConfig(
            tesseractConfig: new TesseractConfig(
                preprocessing: $preprocessing
            )
        )
    );

    $kreuzberg = new Kreuzberg($profileConfig);

    $startTime = microtime(true);
    $result = $kreuzberg->extractFile('sample_scan.pdf');
    $elapsedTime = microtime(true) - $startTime;

    echo ucfirst($profileName) . " profile:\n";
    echo "  Content length: " . strlen($result->content) . " characters\n";
    echo "  Processing time: " . number_format($elapsedTime, 3) . " seconds\n";
    echo "  Settings:\n";
    echo "    - DPI: {$preprocessing->targetDpi}\n";
    echo "    - Denoise: " . ($preprocessing->denoise ? 'Yes' : 'No') . "\n";
    echo "    - Deskew: " . ($preprocessing->deskew ? 'Yes' : 'No') . "\n";
    echo "    - Binarization: " . ($preprocessing->binarizationMethod ?? 'None') . "\n";
    echo "\n";
}

function recommendPreprocessingSettings(string $documentType): ImagePreprocessingConfig
{
    return match ($documentType) {
        'modern_scan' => new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true,
            deskew: true,
            contrastEnhance: false,
            binarizationMethod: 'otsu'
        ),
        'old_document' => new ImagePreprocessingConfig(
            targetDpi: 600,
            denoise: true,
            deskew: true,
            contrastEnhance: true,
            binarizationMethod: 'adaptive',
            removeBackground: true
        ),
        'newspaper' => new ImagePreprocessingConfig(
            targetDpi: 400,
            denoise: true,
            deskew: true,
            contrastEnhance: true,
            binarizationMethod: 'sauvola',
            removeBackground: true
        ),
        default => new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true,
            deskew: true,
            contrastEnhance: true,
            binarizationMethod: 'otsu'
        ),
    };
}

echo "Recommended preprocessing for old documents:\n";
$recommended = recommendPreprocessingSettings('old_document');
echo "  Target DPI: {$recommended->targetDpi}\n";
echo "  Binarization: {$recommended->binarizationMethod}\n";
```
