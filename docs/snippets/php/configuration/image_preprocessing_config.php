```php
<?php

declare(strict_types=1);

/**
 * Image Preprocessing Configuration
 *
 * This example demonstrates image preprocessing options to improve OCR accuracy.
 * Preprocessing can significantly enhance text recognition quality for poor-quality scans.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

echo "Example 1: Default Image Preprocessing\n";
echo "======================================\n";

$config1 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig()
    )
);

echo "Default preprocessing settings:\n";
echo "- Target DPI: 300 (standard for OCR)\n";
echo "- Auto-rotate: Enabled\n";
echo "- Denoise: Disabled\n\n";

echo "Example 2: High DPI Configuration\n";
echo "=================================\n";

$config2 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 600  
        )
    )
);

echo "Target DPI: 600\n";
echo "Best for:\n";
echo "- Very small text\n";
echo "- High-quality scans\n";
echo "- Documents with fine details\n";
echo "Note: Higher DPI = slower processing, more memory\n\n";

echo "Example 3: Lower DPI for Speed\n";
echo "==============================\n";

$config3 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 150  
        )
    )
);

echo "Target DPI: 150\n";
echo "Best for:\n";
echo "- Large text\n";
echo "- Low-resolution images\n";
echo "- Fast processing needed\n";
echo "Note: May reduce accuracy for small text\n\n";

echo "Example 4: Manual Rotation Control\n";
echo "==================================\n";

$config4 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            autoRotate: false  
        )
    )
);

echo "Auto-rotate: Disabled\n";
echo "Use when:\n";
echo "- Images are already correctly oriented\n";
echo "- Auto-rotation causes issues\n";
echo "- Processing time is critical\n\n";

echo "Example 5: Denoising for Poor Quality Scans\n";
echo "===========================================\n";

$config5 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,
            autoRotate: true,
            denoise: true  
        )
    )
);

$kreuzberg = new Kreuzberg($config5);
$result = $kreuzberg->extractFile('noisy_scan.pdf');

echo "Denoising: Enabled\n";
echo "Best for:\n";
echo "- Poor quality scans\n";
echo "- Fax documents\n";
echo "- Images with background noise\n";
echo "- Old or damaged documents\n";
echo "\nExtracted text length: " . strlen($result->content) . " characters\n\n";

echo "Example 6: Maximum Quality Configuration\n";
echo "========================================\n";

$config6 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 600,     
            autoRotate: true,   
            denoise: true       
        )
    )
);

echo "Maximum quality preprocessing:\n";
echo "- Target DPI: 600 (high quality)\n";
echo "- Auto-rotate: Enabled\n";
echo "- Denoise: Enabled\n";
echo "\nBest for:\n";
echo "- Very poor quality scans\n";
echo "- Historical documents\n";
echo "- Faded or damaged text\n";
echo "- Critical accuracy requirements\n\n";

echo "Example 7: Fast Processing Configuration\n";
echo "========================================\n";

$config7 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 200,     
            autoRotate: false,  
            denoise: false      
        )
    )
);

echo "Fast processing configuration:\n";
echo "- Target DPI: 200 (faster)\n";
echo "- Auto-rotate: Disabled\n";
echo "- Denoise: Disabled\n";
echo "\nBest for:\n";
echo "- High-volume processing\n";
echo "- Good quality source images\n";
echo "- Performance-critical applications\n\n";

echo "Example 8: DPI Recommendations by Document Type\n";
echo "===============================================\n";

$standardConfig = new ImagePreprocessingConfig(targetDpi: 300);
echo "Standard documents (letters, reports): 300 DPI\n";

$newspaperConfig = new ImagePreprocessingConfig(targetDpi: 400);
echo "Newspapers and magazines: 400 DPI\n";

$bookConfig = new ImagePreprocessingConfig(targetDpi: 600);
echo "Books with small text: 600 DPI\n";

$receiptConfig = new ImagePreprocessingConfig(targetDpi: 300);
echo "Receipts and forms: 300 DPI\n";

$businessCardConfig = new ImagePreprocessingConfig(targetDpi: 400);
echo "Business cards: 400 DPI\n";

$faxConfig = new ImagePreprocessingConfig(
    targetDpi: 300,
    denoise: true  
);
echo "Faxes: 300 DPI + denoising\n\n";

echo "Example 9: Adaptive Configuration by Image Quality\n";
echo "==================================================\n";

function getPreprocessingConfig(string $quality): ImagePreprocessingConfig
{
    return match ($quality) {
        'excellent' => new ImagePreprocessingConfig(
            targetDpi: 300,
            autoRotate: false,
            denoise: false
        ),
        'good' => new ImagePreprocessingConfig(
            targetDpi: 300,
            autoRotate: true,
            denoise: false
        ),
        'fair' => new ImagePreprocessingConfig(
            targetDpi: 400,
            autoRotate: true,
            denoise: true
        ),
        'poor' => new ImagePreprocessingConfig(
            targetDpi: 600,
            autoRotate: true,
            denoise: true
        ),
        default => new ImagePreprocessingConfig(),
    };
}

echo "Quality-based configurations:\n\n";

echo "Excellent Quality:\n";
echo "- DPI: 300, Auto-rotate: No, Denoise: No\n";
echo "- Clean scans, properly oriented\n\n";

echo "Good Quality:\n";
echo "- DPI: 300, Auto-rotate: Yes, Denoise: No\n";
echo "- May need rotation correction\n\n";

echo "Fair Quality:\n";
echo "- DPI: 400, Auto-rotate: Yes, Denoise: Yes\n";
echo "- Some noise or quality issues\n\n";

echo "Poor Quality:\n";
echo "- DPI: 600, Auto-rotate: Yes, Denoise: Yes\n";
echo "- Significant quality problems\n\n";

echo "Example 10: Complete OCR Pipeline with Preprocessing\n";
echo "===================================================\n";

$config10 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 300,
            autoRotate: true,
            denoise: true
        )
    )
);

$result10 = (new Kreuzberg($config10))->extractFile('poor_quality_scan.pdf');

echo "Processing pipeline:\n";
echo "1. Load image\n";
echo "2. Auto-detect orientation and rotate if needed\n";
echo "3. Upscale/downscale to 300 DPI\n";
echo "4. Apply denoising filter\n";
echo "5. Perform OCR\n";
echo "\nResults:\n";
echo "- Extracted text: " . strlen($result10->content) . " characters\n";
echo "- Pages: " . ($result10->metadata->pageCount ?? 'N/A') . "\n";

echo "\n\nImage Preprocessing Parameters:\n";
echo "================================\n";
echo "- targetDpi: Target resolution in dots per inch\n";
echo "  * 150 DPI: Fast, lower quality\n";
echo "  * 300 DPI: Standard, good balance (RECOMMENDED)\n";
echo "  * 400 DPI: Better for small text\n";
echo "  * 600 DPI: Best quality, slower\n";
echo "\n";
echo "- autoRotate: Automatically detect and correct orientation\n";
echo "  * true: Recommended for most cases\n";
echo "  * false: Skip if images are already oriented\n";
echo "\n";
echo "- denoise: Apply noise reduction filter\n";
echo "  * true: Recommended for poor quality scans\n";
echo "  * false: Skip for clean images (faster)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "1. Start with 300 DPI as a baseline\n";
echo "2. Enable auto-rotate unless you know images are correct\n";
echo "3. Enable denoising for poor quality documents\n";
echo "4. Use higher DPI (400-600) for small text\n";
echo "5. Use lower DPI (150-200) when speed is critical\n";
echo "6. Test different settings to find optimal balance\n";
echo "7. Consider source quality when choosing settings\n";
echo "8. Remember: Higher quality = slower processing + more memory\n";

echo "\n\nPerformance vs Quality Trade-offs:\n";
echo "==================================\n";
echo "Fastest:  DPI=150, AutoRotate=No,  Denoise=No\n";
echo "Balanced: DPI=300, AutoRotate=Yes, Denoise=No  (RECOMMENDED)\n";
echo "Quality:  DPI=400, AutoRotate=Yes, Denoise=Yes\n";
echo "Maximum:  DPI=600, AutoRotate=Yes, Denoise=Yes\n";
```
