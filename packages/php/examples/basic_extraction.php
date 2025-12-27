<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Exceptions\KreuzbergException;

echo "=== Simple Extraction ===\n";
try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/sample.pdf');

    echo "Content length: " . strlen($result->content) . " characters\n";
    echo "MIME type: {$result->mimeType}\n";
    echo "Page count: {$result->metadata->pageCount}\n";
    echo "\nFirst 200 characters:\n";
    echo substr($result->content, 0, 200) . "...\n\n";
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "=== OCR Extraction ===\n";
try {
    $config = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: 'eng'
        )
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/scanned.pdf');

    echo "Content length: " . strlen($result->content) . " characters\n";
    echo "\nFirst 200 characters:\n";
    echo substr($result->content, 0, 200) . "...\n\n";
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "=== Table Extraction ===\n";
try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/tables.pdf');

    echo "Found " . count($result->tables) . " tables\n";

    foreach ($result->tables as $i => $table) {
        echo "\nTable " . ($i + 1) . " (Page {$table->pageNumber}):\n";
        echo $table->markdown . "\n";
    }
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "=== Batch Processing ===\n";
try {
    $files = [
        __DIR__ . '/doc1.pdf',
        __DIR__ . '/doc2.docx',
        __DIR__ . '/doc3.xlsx',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    foreach ($results as $i => $result) {
        echo "File " . ($i + 1) . ": " . strlen($result->content) . " characters\n";
    }
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}
