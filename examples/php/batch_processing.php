<?php

declare(strict_types=1);

/**
 * Batch Processing Example
 *
 * Demonstrates efficient batch processing of multiple documents.
 * Shows how to process files in parallel for maximum performance.
 *
 * This example covers:
 * - Batch file extraction (OOP and procedural API)
 * - Batch extraction from bytes
 * - Processing multiple file formats
 * - Error handling in batch operations
 * - Processing directories
 * - Performance optimization techniques
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\batch_extract_bytes;
use function Kreuzberg\batch_extract_files;


echo "=== Example 1: Simple Batch File Processing (OOP API) ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/document1.pdf',
        __DIR__ . '/../sample-documents/document2.docx',
        __DIR__ . '/../sample-documents/document3.txt',
        __DIR__ . '/../sample-documents/document4.html',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    echo "Processed " . count($results) . " files\n\n";

    foreach ($results as $i => $result) {
        $filename = basename($files[$i]);
        echo "{$filename}:\n";
        echo "  MIME type: {$result->mimeType}\n";
        echo "  Content length: " . strlen($result->content) . " characters\n";
        echo "  Page count: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
        echo "  Tables: " . count($result->tables) . "\n";
        echo "  Preview: " . substr($result->content, 0, 100) . "...\n\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Batch Processing with Procedural API ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/doc1.pdf',
        __DIR__ . '/../sample-documents/doc2.pdf',
        __DIR__ . '/../sample-documents/doc3.pdf',
    ];

    $results = batch_extract_files($files);

    $totalChars = array_reduce(
        $results,
        static fn (int $sum, $result) => $sum + strlen($result->content),
        0
    );
    $totalPages = array_reduce(
        $results,
        static fn (int $sum, $result) => $sum + ($result->metadata->pageCount ?? 0),
        0
    );

    echo "Batch processing statistics:\n";
    echo "  Files processed: " . count($results) . "\n";
    echo "  Total characters: {$totalChars}\n";
    echo "  Total pages: {$totalPages}\n";
    echo "  Average chars per file: " . round($totalChars / count($results)) . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Batch Processing with Configuration ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/scanned1.pdf',
        __DIR__ . '/../sample-documents/scanned2.pdf',
    ];

    $config = new ExtractionConfig(
        ocr: new \Kreuzberg\Config\OcrConfig(
            backend: 'tesseract',
            language: 'eng',
        ),
        extractTables: true,
    );

    $kreuzberg = new Kreuzberg($config);
    $results = $kreuzberg->batchExtractFiles($files);

    echo "Batch OCR processing:\n";
    foreach ($results as $i => $result) {
        echo "  File " . ($i + 1) . ": " . strlen($result->content) . " characters, ";
        echo count($result->tables) . " tables\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Batch Extraction from Bytes ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/doc1.pdf',
        __DIR__ . '/../sample-documents/doc2.docx',
        __DIR__ . '/../sample-documents/doc3.txt',
    ];

    $dataList = [];
    $mimeTypes = [];

    foreach ($files as $file) {
        $data = file_get_contents($file);
        if ($data === false) {
            throw new RuntimeException("Failed to read file: {$file}");
        }

        $dataList[] = $data;

        $extension = strtolower(pathinfo($file, PATHINFO_EXTENSION));
        $mimeTypes[] = match ($extension) {
            'pdf' => 'application/pdf',
            'docx' => 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
            'txt' => 'text/plain',
            'html' => 'text/html',
            'xlsx' => 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
            default => 'application/octet-stream',
        };
    }

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractBytes($dataList, $mimeTypes);

    echo "Batch extraction from bytes (OOP):\n";
    echo "  Processed: " . count($results) . " documents\n\n";

    $results2 = batch_extract_bytes($dataList, $mimeTypes);

    echo "Batch extraction from bytes (Procedural):\n";
    echo "  Processed: " . count($results2) . " documents\n\n";

} catch (KreuzbergException | RuntimeException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: Processing Directory Contents ===\n\n";

try {
    $directory = __DIR__ . '/../sample-documents';

    $pdfFiles = glob($directory . '/*.pdf');

    if (empty($pdfFiles)) {
        echo "No PDF files found in directory\n\n";
    } else {
        $filesToProcess = array_slice($pdfFiles, 0, 5);

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($filesToProcess);

        echo "Processed " . count($results) . " PDF files from directory:\n";

        foreach ($results as $i => $result) {
            $filename = basename($filesToProcess[$i]);
            echo "  {$filename}: " . strlen($result->content) . " characters\n";
        }

        echo "\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Mixed File Type Processing ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/document.pdf',
        __DIR__ . '/../sample-documents/spreadsheet.xlsx',
        __DIR__ . '/../sample-documents/presentation.pptx',
        __DIR__ . '/../sample-documents/article.docx',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    echo "Processing different file types:\n";

    $typeStats = [];
    foreach ($results as $i => $result) {
        $type = $result->mimeType;

        if (!isset($typeStats[$type])) {
            $typeStats[$type] = [
                'count' => 0,
                'chars' => 0,
                'tables' => 0,
            ];
        }

        $typeStats[$type]['count']++;
        $typeStats[$type]['chars'] += strlen($result->content);
        $typeStats[$type]['tables'] += count($result->tables);
    }

    echo "\nStatistics by file type:\n";
    foreach ($typeStats as $type => $stats) {
        echo "  {$type}:\n";
        echo "    Files: {$stats['count']}\n";
        echo "    Total characters: {$stats['chars']}\n";
        echo "    Total tables: {$stats['tables']}\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Error Handling in Batch Processing ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/valid1.pdf',
        __DIR__ . '/nonexistent.pdf',
        __DIR__ . '/../sample-documents/valid2.txt',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    echo "All files processed successfully\n";

} catch (KreuzbergException $e) {
    echo "Batch processing failed:\n";
    echo "  Error: {$e->getMessage()}\n";
    echo "\nNote: Batch operations fail fast on first error.\n";
    echo "For better error handling, process files individually:\n\n";

    foreach ($files as $file) {
        $filename = basename($file);
        try {
            $kreuzberg = new Kreuzberg();
            $result = $kreuzberg->extractFile($file);
            echo "  OK: {$filename} - " . strlen($result->content) . " characters\n";
        } catch (KreuzbergException $e) {
            echo "  ERROR: {$filename} - {$e->getMessage()}\n";
        }
    }

    echo "\n";
}


echo "=== Example 8: Performance Optimization ===\n\n";

try {
    $files = [];
    for ($i = 1; $i <= 10; $i++) {
        $files[] = __DIR__ . "/../sample-documents/doc{$i}.pdf";
    }

    $startTime = microtime(true);

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    $batchTime = microtime(true) - $startTime;

    echo "Batch processing performance:\n";
    echo "  Files: " . count($results) . "\n";
    echo "  Time: " . round($batchTime, 3) . " seconds\n";
    echo "  Average per file: " . round($batchTime / count($results), 3) . " seconds\n";

    $startTime = microtime(true);

    foreach (array_slice($files, 0, 3) as $file) {
        $kreuzberg->extractFile($file);
    }

    $sequentialTime = microtime(true) - $startTime;

    echo "\nSequential processing (first 3 files):\n";
    echo "  Time: " . round($sequentialTime, 3) . " seconds\n";
    echo "  Average per file: " . round($sequentialTime / 3, 3) . " seconds\n";

    echo "\nSpeedup: " . round(($sequentialTime / 3) / ($batchTime / count($results)), 2) . "x faster\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 9: Batch Processing with Result Filtering ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/doc1.pdf',
        __DIR__ . '/../sample-documents/doc2.pdf',
        __DIR__ . '/../sample-documents/doc3.pdf',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    $longDocuments = array_filter(
        $results,
        static fn ($result) => strlen($result->content) > 1000
    );

    echo "Long documents (>1000 characters): " . count($longDocuments) . "\n";

    $documentsWithTables = array_filter(
        $results,
        static fn ($result) => count($result->tables) > 0
    );

    echo "Documents with tables: " . count($documentsWithTables) . "\n";

    $pdfDocuments = array_filter(
        $results,
        static fn ($result) => $result->mimeType === 'application/pdf'
    );

    echo "PDF documents: " . count($pdfDocuments) . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "Done!\n";
