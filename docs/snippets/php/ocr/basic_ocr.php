```php title="basic_ocr.php"
<?php

declare(strict_types=1);

/**
 * Basic OCR with Tesseract
 *
 * Extract text from scanned PDFs and images using Tesseract OCR.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "OCR Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->getContent() . "\n\n";

$multilingualConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+fra+deu'  
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('multilingual_scan.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Multilingual OCR:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->getContent(), 0, 500) . "...\n\n";

$imageConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);


$imageFormats = ['png', 'jpg', 'tiff'];
foreach ($imageFormats as $format) {
    $file = "scan.$format";
    if (file_exists($file)) {
        echo "Processing $file...\n";
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        echo "Extracted " . strlen($result->getContent()) . " characters\n";
        echo "Preview: " . substr($result->getContent(), 0, 100) . "...\n\n";
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

        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

        echo "$description ($lang):\n";
        echo "  Characters extracted: " . mb_strlen($result->getContent()) . "\n\n";
    }
}


$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('invoice_scan.pdf'), $config);
$result = $output->results[0];

echo "Invoice OCR:\n";
echo str_repeat('=', 60) . "\n";
echo $result->getContent() . "\n";

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('scanned.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

$contentLength = strlen($result->getContent());
$pageCount = $result->metadata?->pdf?->page_count ?? 1;
$avgCharsPerPage = $contentLength / $pageCount;

echo "\nOCR Quality Assessment:\n";
echo "Total characters: $contentLength\n";
echo "Pages: $pageCount\n";
echo "Average chars/page: " . number_format($avgCharsPerPage) . "\n";

if ($avgCharsPerPage < 100) {
    echo "Warning: Low character count may indicate poor scan quality\n";
    echo "Consider using image preprocessing or higher DPI settings.\n";
} elseif ($avgCharsPerPage > 2000) {
    echo "Pass: Good - Adequate text extracted\n";
} else {
    echo "Pass: Moderate - Text extracted successfully\n";
}
```
