```php title="image_preprocessing.php"
<?php

declare(strict_types=1);

/**
 * Image Preprocessing for OCR
 *
 * Improve OCR accuracy by preprocessing images before text recognition.
 * Useful for poor quality scans, photos, and challenging documents.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;
use Xberg\Config\ImagePreprocessingConfig;

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('noisy_scan.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Basic Preprocessing Results:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->getContent(), 0, 300) . "...\n\n";

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('small_text_scan.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "High DPI Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Characters extracted: " . strlen($result->getContent()) . "\n";
echo "Preview: " . substr($result->getContent(), 0, 200) . "...\n\n";

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('crooked_scan.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Deskewed OCR Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->getContent() . "\n\n";

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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('watermarked_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Background Removal Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Extracted " . strlen($result->getContent()) . " characters\n";
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

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('very_poor_quality.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Comprehensive Preprocessing:\n";
echo str_repeat('=', 60) . "\n";
echo "Original quality: Very Poor\n";
echo "After preprocessing:\n";
echo "  Characters: " . strlen($result->getContent()) . "\n";
echo "  Content preview:\n";
echo "  " . substr($result->getContent(), 0, 300) . "...\n\n";

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
        $start = microtime(true);
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($testFile), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        $elapsed = microtime(true) - $start;

        echo "$name:\n";
        echo "  Time: " . number_format($elapsed, 3) . "s\n";
        echo "  Characters: " . strlen($result->getContent()) . "\n";
        echo "  Tables: " . count($result->tables) . "\n\n";
    }
}

function getOptimalPreprocessing(string $file): ImagePreprocessingConfig
{
    $quickScan = new Xberg(new ExtractionConfig(
        ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
    ));
    $quickResult = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default())->results[0];

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

    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

    echo "Adaptive preprocessing applied\n";
    echo "Result: " . strlen($result->getContent()) . " characters extracted\n";
}
```
