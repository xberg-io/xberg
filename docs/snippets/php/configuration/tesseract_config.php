```php
<?php

declare(strict_types=1);

/**
 * Tesseract OCR Configuration
 *
 * This example demonstrates advanced Tesseract OCR configuration options
 * for fine-tuning OCR performance and accuracy.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;

echo "Example 1: Default Tesseract Configuration\n";
echo "==========================================\n";

$config1 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig()  
    )
);

echo "Default settings:\n";
echo "- PSM: 3 (Fully automatic page segmentation)\n";
echo "- OEM: 3 (Default, based on what's available)\n";
echo "- Table Detection: Disabled\n\n";

echo "Example 2: Different Page Segmentation Modes\n";
echo "============================================\n";

$config2a = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(psm: 6)
    )
);

echo "PSM 6 - Uniform block of text:\n";
echo "- Best for: Most documents, clean text blocks\n";
echo "- Use when: Document has clear text structure\n\n";

$config2b = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(psm: 11)
    )
);

echo "PSM 11 - Sparse text:\n";
echo "- Best for: Screenshots, signs, sparse documents\n";
echo "- Use when: Text is scattered across the image\n\n";

$config2c = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(psm: 7)
    )
);

echo "PSM 7 - Single text line:\n";
echo "- Best for: Single line of text, headers, captions\n";
echo "- Use when: Processing individual text lines\n\n";

$config2d = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(psm: 8)
    )
);

echo "PSM 8 - Single word:\n";
echo "- Best for: Individual words, labels\n";
echo "- Use when: Processing single words\n\n";

echo "Example 3: Table Detection\n";
echo "=========================\n";

$config3 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            enableTableDetection: true  
        )
    )
);

$kreuzberg = new Kreuzberg($config3);
$result = $kreuzberg->extractFile('scanned_invoice.pdf');

echo "Table detection enabled\n";
echo "Best for: Forms, invoices, spreadsheets, reports\n";

if (count($result->tables) > 0) {
    echo "\nExtracted tables: " . count($result->tables) . "\n";
    foreach ($result->tables as $i => $table) {
        echo "\nTable " . ($i + 1) . ":\n";
        echo $table->markdown . "\n";
    }
}

echo "\n\n";

echo "Example 4: Character Whitelisting\n";
echo "=================================\n";

$config4a = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            tesseditCharWhitelist: '0123456789'  
        )
    )
);

echo "Whitelist: '0123456789' (digits only)\n";
echo "Best for: Serial numbers, IDs, numeric codes\n\n";

$config4b = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            tesseditCharWhitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
        )
    )
);

echo "Whitelist: Letters and numbers only\n";
echo "Best for: Product codes, alphanumeric IDs\n\n";

$config4c = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            tesseditCharWhitelist: '0123456789$€£¥.,- '
        )
    )
);

echo "Whitelist: '0123456789$€£¥.,- ' (financial data)\n";
echo "Best for: Invoices, receipts, price lists\n\n";

echo "Example 5: Character Blacklisting\n";
echo "=================================\n";

$config5 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            tesseditCharBlacklist: '|!@#%^&*()'  
        )
    )
);

echo "Blacklist: '|!@#%^&*()'\n";
echo "Use to: Exclude problematic characters that cause OCR errors\n\n";

echo "Example 6: OCR Engine Modes\n";
echo "===========================\n";

$config6a = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(oem: 0)
    )
);

echo "OEM 0 - Legacy engine:\n";
echo "- Older, simpler algorithm\n";
echo "- Sometimes better for very low-quality scans\n\n";

$config6b = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(oem: 1)
    )
);

echo "OEM 1 - LSTM neural network:\n";
echo "- Modern deep learning approach\n";
echo "- Better accuracy for most documents\n\n";

$config6c = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(oem: 3)
    )
);

echo "OEM 3 - Default (recommended):\n";
echo "- Chooses best available engine\n";
echo "- Use this unless you have specific needs\n\n";

echo "Example 7: Complete Invoice Processing Configuration\n";
echo "====================================================\n";

$config7 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,                      
            oem: 3,                      
            enableTableDetection: true,  
            tesseditCharWhitelist: '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz$€£.,- :#/'
        )
    )
);

echo "Invoice-optimized configuration:\n";
echo "- PSM 6: Structured text\n";
echo "- Table detection: Enabled\n";
echo "- Character whitelist: Alphanumeric + currency + common symbols\n";
echo "- Best for: Invoices, receipts, financial documents\n\n";

echo "Example 8: Complete Form Processing Configuration\n";
echo "=================================================\n";

$config8 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 6,
            oem: 3,
            enableTableDetection: true,
            tesseditCharWhitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,- @'
        )
    )
);

echo "Form-optimized configuration:\n";
echo "- PSM 6: Structured text\n";
echo "- Table detection: Enabled\n";
echo "- Character whitelist: Alphanumeric + common form characters\n";
echo "- Best for: Forms, applications, surveys\n\n";

echo "Example 9: Sparse Text Configuration\n";
echo "====================================\n";

$config9 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 11,  
            oem: 3
        )
    )
);

echo "Sparse text configuration:\n";
echo "- PSM 11: Find scattered text\n";
echo "- Best for: Screenshots, signs, posters, sparse documents\n\n";

echo "\nAll Page Segmentation Modes:\n";
echo "============================\n";
echo "0  = OSD only (orientation and script detection)\n";
echo "1  = Automatic page segmentation with OSD\n";
echo "2  = Automatic page segmentation (no OSD or OCR)\n";
echo "3  = Fully automatic page segmentation (default)\n";
echo "4  = Single column of variable-sized text\n";
echo "5  = Single uniform block of vertically aligned text\n";
echo "6  = Single uniform block of text (RECOMMENDED)\n";
echo "7  = Single text line\n";
echo "8  = Single word\n";
echo "9  = Single word in a circle\n";
echo "10 = Single character\n";
echo "11 = Sparse text (RECOMMENDED for screenshots)\n";
echo "12 = Sparse text with OSD\n";
echo "13 = Raw line\n";

echo "\n\nOCR Engine Modes:\n";
echo "=================\n";
echo "0 = Legacy engine only\n";
echo "1 = LSTM neural network only\n";
echo "2 = Legacy + LSTM\n";
echo "3 = Default (RECOMMENDED)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "1. Start with PSM 6 and OEM 3 (defaults)\n";
echo "2. Use PSM 11 for sparse/scattered text\n";
echo "3. Enable table detection for structured documents\n";
echo "4. Use character whitelists for constrained input\n";
echo "5. Use blacklists to exclude problem characters\n";
echo "6. Test different PSM values if accuracy is poor\n";
echo "7. Combine with image preprocessing for better results\n";
```
