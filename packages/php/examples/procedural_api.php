<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;
use function Kreuzberg\batch_extract_files;
use function Kreuzberg\detect_mime_type;
use function Kreuzberg\detect_mime_type_from_path;
use function Kreuzberg\extract_bytes;
use function Kreuzberg\extract_file;

echo "=== Simple File Extraction ===\n";

$result = extract_file(__DIR__ . '/document.pdf');
echo "Content length: " . strlen($result->content) . " characters\n";
echo "Title: {$result->metadata->title}\n";
echo "Page count: {$result->metadata->pageCount}\n\n";

echo "=== OCR Extraction ===\n";

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

$result = extract_file(__DIR__ . '/invoice.pdf', config: $config);
echo "Content length: " . strlen($result->content) . " characters\n";
echo "Tables found: " . count($result->tables) . "\n\n";

echo "=== Extract from Bytes ===\n";

$data = file_get_contents(__DIR__ . '/document.pdf');
$result = extract_bytes($data, 'application/pdf');
echo "Content length: " . strlen($result->content) . " characters\n\n";

echo "=== Batch Extraction ===\n";

$files = [
    __DIR__ . '/doc1.pdf',
    __DIR__ . '/doc2.docx',
    __DIR__ . '/doc3.xlsx',
];

$results = batch_extract_files($files);

foreach ($results as $i => $result) {
    echo "File " . ($i + 1) . ":\n";
    echo "  MIME type: {$result->mimeType}\n";
    echo "  Content length: " . strlen($result->content) . " characters\n";
}

echo "\n";

echo "=== MIME Type Detection ===\n";

$data = file_get_contents(__DIR__ . '/unknown.file');
$mimeType = detect_mime_type($data);
echo "Detected MIME type from bytes: {$mimeType}\n";

$mimeType = detect_mime_type_from_path(__DIR__ . '/document.pdf');
echo "Detected MIME type from path: {$mimeType}\n";
