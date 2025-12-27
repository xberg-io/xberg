```php
<?php

declare(strict_types=1);

/**
 * Basic OCR with Tesseract
 *
 * Extract text from scanned PDFs and images using Tesseract OCR.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "OCR Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$multilingualConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+fra+deu'  
    )
);

$kreuzberg = new Kreuzberg($multilingualConfig);
$result = $kreuzberg->extractFile('multilingual_scan.pdf');

echo "Multilingual OCR:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->content, 0, 500) . "...\n\n";

$imageConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($imageConfig);

$imageFormats = ['png', 'jpg', 'tiff'];
foreach ($imageFormats as $format) {
    $file = "scan.$format";
    if (file_exists($file)) {
        echo "Processing $file...\n";
        $result = $kreuzberg->extractFile($file);
        echo "Extracted " . strlen($result->content) . " characters\n";
        echo "Preview: " . substr($result->content, 0, 100) . "...\n\n";
    }
}

$languages = [
    'spa' => 'Spanish document',
    'fra' => 'French document',
    'deu' => 'German document',
    'ita' => 'Italian document',
    'por' => 'Portuguese document',
    'rus' => 'Russian document',
    'jpn' => 'Japanese document',
    'chi_sim' => 'Chinese (Simplified) document',
];

foreach ($languages as $lang => $description) {
    $file = strtolower(str_replace(' ', '_', $description)) . '.pdf';

    if (file_exists($file)) {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: $lang
            )
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        echo "$description ($lang):\n";
        echo "  Characters extracted: " . mb_strlen($result->content) . "\n\n";
    }
}

use function Kreuzberg\extract_file;

$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

$result = extract_file('invoice_scan.pdf', config: $config);

echo "Invoice OCR:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n";

$result = $kreuzberg->extractFile('scanned.pdf');

$contentLength = strlen($result->content);
$pageCount = $result->metadata->pageCount ?? 1;
$avgCharsPerPage = $contentLength / $pageCount;

echo "\nOCR Quality Assessment:\n";
echo "Total characters: $contentLength\n";
echo "Pages: $pageCount\n";
echo "Average chars/page: " . number_format($avgCharsPerPage) . "\n";

if ($avgCharsPerPage < 100) {
    echo "⚠ Warning: Low character count may indicate poor scan quality\n";
    echo "Consider using image preprocessing or higher DPI settings.\n";
} elseif ($avgCharsPerPage > 2000) {
    echo "✓ Good: Adequate text extracted\n";
} else {
    echo "✓ Moderate: Text extracted successfully\n";
}
```
