<?php

declare(strict_types=1);

/**
 * Basic Usage Example
 *
 * Demonstrates basic document extraction with Kreuzberg.
 * Shows both OOP and procedural API usage for simple extraction tasks.
 *
 * This example covers:
 * - Simple file extraction
 * - Extraction with configuration
 * - Extract from bytes
 * - MIME type detection
 * - Accessing metadata and content
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\detect_mime_type;
use function Kreuzberg\detect_mime_type_from_path;
use function Kreuzberg\extract_bytes;
use function Kreuzberg\extract_file;


echo "=== Example 1: Simple File Extraction (OOP API) ===\n\n";

try {
    $kreuzberg = new Kreuzberg();

    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Content length: " . strlen($result->content) . " characters\n";
    echo "MIME type: {$result->mimeType}\n";
    echo "Page count: {$result->metadata->pageCount}\n";

    echo "\nFirst 200 characters:\n";
    echo substr($result->content, 0, 200) . "...\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Simple File Extraction (Procedural API) ===\n\n";

try {
    $result = extract_file(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Content length: " . strlen($result->content) . " characters\n";
    echo "MIME type: {$result->mimeType}\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Extraction with Configuration ===\n\n";

try {
    $config = new ExtractionConfig(
        extractTables: true,
        extractImages: true,
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Extracted with tables and images: " . strlen($result->content) . " characters\n";
    echo "Tables found: " . count($result->tables) . "\n";

    $specificConfig = new ExtractionConfig(
        extractTables: false,
        extractImages: false,
    );
    $result2 = $kreuzberg->extractFile(
        __DIR__ . '/../sample-documents/sample.pdf',
        config: $specificConfig
    );

    echo "Extracted without quality processing: " . strlen($result2->content) . " characters\n";

    $result3 = extract_file(
        __DIR__ . '/../sample-documents/sample.pdf',
        config: $config
    );

    echo "Procedural API with config: " . strlen($result3->content) . " characters\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Extract from Bytes ===\n\n";

try {
    $data = file_get_contents(__DIR__ . '/../sample-documents/sample.pdf');

    if ($data === false) {
        throw new RuntimeException('Failed to read file');
    }

    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractBytes($data, 'application/pdf');

    echo "OOP API - Extracted from bytes: " . strlen($result->content) . " characters\n";

    $result2 = extract_bytes($data, 'application/pdf');

    echo "Procedural API - Extracted from bytes: " . strlen($result2->content) . " characters\n\n";

} catch (KreuzbergException | RuntimeException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: MIME Type Detection ===\n\n";

try {
    $mimeType = detect_mime_type_from_path(__DIR__ . '/../sample-documents/sample.pdf');
    echo "Detected MIME type from path: {$mimeType}\n";

    $data = file_get_contents(__DIR__ . '/../sample-documents/sample.pdf');

    if ($data === false) {
        throw new RuntimeException('Failed to read file');
    }

    $mimeType = detect_mime_type($data);
    echo "Detected MIME type from bytes: {$mimeType}\n\n";

} catch (RuntimeException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Accessing Metadata ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Document Metadata:\n";
    echo "  Title: " . ($result->metadata->title ?? 'N/A') . "\n";
    echo "  Subject: " . ($result->metadata->subject ?? 'N/A') . "\n";
    echo "  Authors: " . (
        $result->metadata->authors
            ? implode(', ', $result->metadata->authors)
            : 'N/A'
    ) . "\n";
    echo "  Created at: " . ($result->metadata->createdAt ?? 'N/A') . "\n";
    echo "  Modified at: " . ($result->metadata->modifiedAt ?? 'N/A') . "\n";
    echo "  Created by: " . ($result->metadata->createdBy ?? 'N/A') . "\n";
    echo "  Producer: " . ($result->metadata->producer ?? 'N/A') . "\n";
    echo "  Page count: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
    echo "  Language: " . ($result->metadata->language ?? 'N/A') . "\n";
    echo "  Format type: " . ($result->metadata->formatType ?? 'N/A') . "\n";

    if (!empty($result->metadata->custom)) {
        echo "\nCustom Metadata:\n";
        foreach ($result->metadata->custom as $key => $value) {
            echo "  {$key}: " . print_r($value, true) . "\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Accessing Tables ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Found " . count($result->tables) . " tables\n";

    foreach ($result->tables as $i => $table) {
        echo "\nTable " . ($i + 1) . ":\n";
        echo "  Page: {$table->pageNumber}\n";
        echo "  Rows: {$table->rowCount}\n";
        echo "  Columns: {$table->columnCount}\n";
        echo "\n  Markdown:\n";
        echo "  " . str_replace("\n", "\n  ", $table->markdown) . "\n";

        if ($table->data !== null) {
            echo "\n  First row (array format):\n";
            echo "  " . print_r($table->data[0] ?? [], true);
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 8: Error Handling ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/nonexistent.pdf');

    echo "Content: {$result->content}\n";

} catch (KreuzbergException $e) {
    echo "Caught expected error:\n";
    echo "  Message: {$e->getMessage()}\n";
    echo "  Code: {$e->getCode()}\n\n";
}

echo "Done!\n";
