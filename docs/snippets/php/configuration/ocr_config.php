```php
<?php

declare(strict_types=1);

/**
 * OCR Configuration
 *
 * This example demonstrates how to configure OCR (Optical Character Recognition)
 * for extracting text from scanned documents and images.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;

echo "Example 1: Basic OCR Configuration\n";
echo "==================================\n";

$config1 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'  
    )
);

$kreuzberg = new Kreuzberg($config1);
$result = $kreuzberg->extractFile('scanned_document.pdf');
echo "Extracted text length: " . strlen($result->content) . " characters\n\n";

echo "Example 2: Multi-Language OCR\n";
echo "=============================\n";

$config2 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+fra+deu'  
    )
);

echo "Configured for languages: English, French, German\n";
echo "Use this for multilingual documents\n\n";

echo "Example 3: Language-Specific OCR\n";
echo "================================\n";

$config3a = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'spa')
);

$config3b = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'fra')
);

$config3c = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'deu')
);

$config3d = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'chi_sim')
);

$config3e = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'chi_tra')
);

$config3f = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'jpn')
);

$config3g = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'kor')
);

$config3h = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'ara')
);

echo "Common Tesseract Language Codes:\n";
echo "- eng: English\n";
echo "- fra: French\n";
echo "- deu: German\n";
echo "- spa: Spanish\n";
echo "- ita: Italian\n";
echo "- por: Portuguese\n";
echo "- rus: Russian\n";
echo "- chi_sim: Chinese (Simplified)\n";
echo "- chi_tra: Chinese (Traditional)\n";
echo "- jpn: Japanese\n";
echo "- kor: Korean\n";
echo "- ara: Arabic\n\n";

echo "Example 4: Advanced Tesseract Configuration\n";
echo "==========================================\n";

$config4 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,                     
            oem: 3,                     
            enableTableDetection: true  
        )
    )
);

echo "Tesseract Configuration:\n";
echo "- PSM (Page Segmentation Mode): 6 (uniform text block)\n";
echo "- OEM (OCR Engine Mode): 3 (LSTM only)\n";
echo "- Table Detection: Enabled\n\n";

echo "Example 5: OCR for Forms and Invoices\n";
echo "=====================================\n";

$config5 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,                      
            oem: 3,                      
            enableTableDetection: true,  
            tesseditCharWhitelist: '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz$.,- '
        )
    )
);

echo "Optimized for forms and invoices:\n";
echo "- Table detection enabled\n";
echo "- Character whitelist for common form characters\n\n";

echo "Example 6: OCR for Numeric Documents\n";
echo "====================================\n";

$config6 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            oem: 3,
            tesseditCharWhitelist: '0123456789$.,- '  
        )
    )
);

echo "Character whitelist: '0123456789$.,- '\n";
echo "Best for: Invoices, receipts, financial documents\n\n";

echo "Example 7: OCR with Character Blacklist\n";
echo "=======================================\n";

$config7 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            oem: 3,
            tesseditCharBlacklist: '|!@#%^&*()'  
        )
    )
);

echo "Character blacklist: '|!@#%^&*()'\n";
echo "Use to exclude problematic characters\n\n";

echo "\nPage Segmentation Modes (PSM):\n";
echo "==============================\n";
echo "0  = Orientation and script detection (OSD) only\n";
echo "1  = Automatic page segmentation with OSD\n";
echo "2  = Automatic page segmentation (no OSD or OCR)\n";
echo "3  = Fully automatic page segmentation (default)\n";
echo "4  = Assume a single column of text of variable sizes\n";
echo "5  = Assume a single uniform block of vertically aligned text\n";
echo "6  = Assume a single uniform block of text (recommended for most)\n";
echo "7  = Treat the image as a single text line\n";
echo "8  = Treat the image as a single word\n";
echo "9  = Treat the image as a single word in a circle\n";
echo "10 = Treat the image as a single character\n";
echo "11 = Sparse text. Find as much text as possible\n";
echo "12 = Sparse text with OSD\n";
echo "13 = Raw line. Treat the image as a single text line\n";

echo "\n\nOCR Engine Modes (OEM):\n";
echo "======================\n";
echo "0 = Legacy engine only\n";
echo "1 = Neural nets LSTM engine only\n";
echo "2 = Legacy + LSTM engines\n";
echo "3 = Default, based on what is available (recommended)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "- Use PSM 6 for general documents\n";
echo "- Use PSM 11 for sparse text (screenshots, signs)\n";
echo "- Use OEM 3 (default) for best results\n";
echo "- Enable table detection for structured documents\n";
echo "- Use character whitelists for forms/invoices\n";
echo "- Combine multiple languages with '+' separator\n";
echo "- Preprocess images for better accuracy (see image_preprocessing.php)\n";
```
