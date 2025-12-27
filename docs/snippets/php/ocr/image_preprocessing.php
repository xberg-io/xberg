```php
<?php

declare(strict_types=1);

/**
 * Image Preprocessing for OCR
 *
 * Improve OCR accuracy by preprocessing images before text recognition.
 * Useful for poor quality scans, photos, and challenging documents.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            denoise: true,
            sharpen: true
        )
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('noisy_scan.pdf');

echo "Basic Preprocessing Results:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->content, 0, 300) . "...\n\n";

$highDpiConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 400,  
            denoise: true,
            sharpen: true
        )
    )
);

$kreuzberg = new Kreuzberg($highDpiConfig);
$result = $kreuzberg->extractFile('small_text_scan.pdf');

echo "High DPI Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Characters extracted: " . strlen($result->content) . "\n";
echo "Preview: " . substr($result->content, 0, 200) . "...\n\n";

$deskewConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            deskew: true,      
            autoRotate: true,  
            targetDpi: 300
        )
    )
);

$kreuzberg = new Kreuzberg($deskewConfig);
$result = $kreuzberg->extractFile('crooked_scan.pdf');

echo "Deskewed OCR Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$cleanConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            removeBackground: true,
            denoise: true,
            targetDpi: 300
        )
    )
);

$kreuzberg = new Kreuzberg($cleanConfig);
$result = $kreuzberg->extractFile('watermarked_document.pdf');

echo "Background Removal Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Extracted " . strlen($result->content) . " characters\n";
echo "Text quality improved by removing background noise\n\n";

$comprehensiveConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 400,         
            denoise: true,          
            sharpen: true,          
            autoRotate: true,       
            deskew: true,           
            removeBackground: true, 
            contrastEnhancement: true,  
            binarize: true         
        )
    )
);

$kreuzberg = new Kreuzberg($comprehensiveConfig);
$result = $kreuzberg->extractFile('very_poor_quality.pdf');

echo "Comprehensive Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Original quality: Very Poor\n";
echo "After preprocessing:\n";
echo "  Characters: " . strlen($result->content) . "\n";
echo "  Content preview:\n";
echo "  " . substr($result->content, 0, 300) . "...\n\n";

$testFile = 'test_scan.pdf';
if (file_exists($testFile)) {
    $configs = [
        'No preprocessing' => new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
        ),
        'Denoise only' => new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
                imagePreprocessing: new ImagePreprocessingConfig(denoise: true)
            )
        ),
        'Denoise + Sharpen' => new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
                imagePreprocessing: new ImagePreprocessingConfig(
                    denoise: true,
                    sharpen: true
                )
            )
        ),
        'Full preprocessing' => $comprehensiveConfig,
    ];

    echo "Preprocessing Comparison:\n";
    echo str_repeat('=', 60) . "\n";

    foreach ($configs as $name => $config) {
        $kreuzberg = new Kreuzberg($config);
        $start = microtime(true);
        $result = $kreuzberg->extractFile($testFile);
        $elapsed = microtime(true) - $start;

        echo "$name:\n";
        echo "  Time: " . number_format($elapsed, 3) . "s\n";
        echo "  Characters: " . strlen($result->content) . "\n";
        echo "  Tables: " . count($result->tables) . "\n\n";
    }
}

function getOptimalPreprocessing(string $file): ImagePreprocessingConfig
{
    $quickScan = new Kreuzberg(new ExtractionConfig(
        ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
    ));
    $quickResult = $quickScan->extractFile($file);

    $fileSize = filesize($file);
    $contentLength = strlen($quickResult->content);
    $ratio = $contentLength / $fileSize;

    if ($ratio < 0.01) {
        return new ImagePreprocessingConfig(
            targetDpi: 400,
            denoise: true,
            sharpen: true,
            autoRotate: true,
            deskew: true,
            removeBackground: true,
            contrastEnhancement: true
        );
    } elseif ($ratio < 0.05) {
        return new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true,
            sharpen: true,
            deskew: true
        );
    } else {
        return new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true
        );
    }
}

$file = 'auto_detect_quality.pdf';
if (file_exists($file)) {
    $preprocessing = getOptimalPreprocessing($file);

    $config = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: 'eng',
            imagePreprocessing: $preprocessing
        )
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($file);

    echo "Adaptive preprocessing applied\n";
    echo "Result: " . strlen($result->content) . " characters extracted\n";
}
```
