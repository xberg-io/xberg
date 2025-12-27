<?php

declare(strict_types=1);

/**
 * Metadata Extraction Example
 *
 * Demonstrates extracting detailed metadata from various document types.
 * Shows how to access and work with document metadata, tables, images, and pages.
 *
 * This example covers:
 * - Document metadata extraction
 * - PDF-specific metadata
 * - Office document metadata
 * - Table extraction and analysis
 * - Image extraction with metadata
 * - Page-level metadata
 * - Custom metadata fields
 * - Language detection
 * - Keyword extraction
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_file;


echo "=== Example 1: Basic Metadata Extraction ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Document Metadata:\n";
    echo "  Title: " . ($result->metadata->title ?? 'N/A') . "\n";
    echo "  Subject: " . ($result->metadata->subject ?? 'N/A') . "\n";
    echo "  Language: " . ($result->metadata->language ?? 'N/A') . "\n";
    echo "  Date: " . ($result->metadata->date ?? 'N/A') . "\n";
    echo "  Format Type: " . ($result->metadata->formatType ?? 'N/A') . "\n";
    echo "  Page Count: " . ($result->metadata->pageCount ?? 'N/A') . "\n";

    if ($result->metadata->authors !== null) {
        echo "  Authors: " . implode(', ', $result->metadata->authors) . "\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Detailed Metadata Fields ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    $metadata = $result->metadata;

    echo "Complete Metadata:\n";
    echo "  Title: " . ($metadata->title ?? 'N/A') . "\n";
    echo "  Subject: " . ($metadata->subject ?? 'N/A') . "\n";
    echo "  Language: " . ($metadata->language ?? 'N/A') . "\n";
    echo "  Date: " . ($metadata->date ?? 'N/A') . "\n";
    echo "  Format Type: " . ($metadata->formatType ?? 'N/A') . "\n";

    if ($metadata->authors !== null) {
        echo "  Authors:\n";
        foreach ($metadata->authors as $author) {
            echo "    - {$author}\n";
        }
    }

    if ($metadata->keywords !== null) {
        echo "  Keywords: " . implode(', ', $metadata->keywords) . "\n";
    }

    echo "\nTimestamp Information:\n";
    echo "  Created At: " . ($metadata->createdAt ?? 'N/A') . "\n";
    echo "  Modified At: " . ($metadata->modifiedAt ?? 'N/A') . "\n";

    echo "\nCreator Information:\n";
    echo "  Created By: " . ($metadata->createdBy ?? 'N/A') . "\n";
    echo "  Producer: " . ($metadata->producer ?? 'N/A') . "\n";

    echo "\nDocument Statistics:\n";
    echo "  Page Count: " . ($metadata->pageCount ?? 'N/A') . "\n";
    echo "  Content Length: " . strlen($result->content) . " characters\n";
    echo "  Tables: " . count($result->tables) . "\n";

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Custom Metadata Fields ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    echo "Custom Metadata:\n";

    if (!empty($result->metadata->custom)) {
        foreach ($result->metadata->custom as $key => $value) {
            $displayValue = is_array($value)
                ? json_encode($value)
                : (string) $value;

            echo "  {$key}: {$displayValue}\n";
        }
    } else {
        echo "  No custom metadata fields found\n";
    }

    if ($result->metadata->hasCustom('custom_field')) {
        $value = $result->metadata->getCustom('custom_field');
        echo "\nCustom field 'custom_field': {$value}\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Table Metadata ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/tables.pdf');

    echo "Table Extraction:\n";
    echo "  Total tables: " . count($result->tables) . "\n\n";

    foreach ($result->tables as $i => $table) {
        echo "Table " . ($i + 1) . " Metadata:\n";
        echo "  Page Number: {$table->pageNumber}\n";
        echo "  Rows: " . count($table->cells) . "\n";
        echo "  Columns: " . (count($table->cells) > 0 ? count($table->cells[0]) : 0) . "\n";

        echo "\n  Markdown representation:\n";
        echo "  " . str_replace("\n", "\n  ", $table->markdown) . "\n";

        echo "\n  Cell data (first 3 rows):\n";
        foreach (array_slice($table->cells, 0, 3) as $rowIdx => $row) {
            echo "    Row {$rowIdx}: " . implode(' | ', $row) . "\n";
        }

        echo "\n";
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: Image Metadata ===\n\n";

try {
    $config = new ExtractionConfig(
        imageExtraction: new ImageExtractionConfig(
            extractImages: true,
            performOcr: false,
            minWidth: 50,
            minHeight: 50,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/presentation.pptx');

    echo "Image Extraction:\n";
    echo "  Total images: " . count($result->images ?? []) . "\n\n";

    if ($result->images !== null) {
        foreach (array_slice($result->images, 0, 5) as $image) {
            echo "Image {$image->imageIndex} Metadata:\n";
            echo "  Format: {$image->format}\n";
            echo "  Page: " . ($image->pageNumber ?? 'N/A') . "\n";
            echo "  Size: " . ($image->width ?? 'N/A') . "x" . ($image->height ?? 'N/A') . " pixels\n";
            echo "  Colorspace: " . ($image->colorspace ?? 'N/A') . "\n";
            echo "  Bits per component: " . ($image->bitsPerComponent ?? 'N/A') . "\n";
            echo "  Is mask: " . ($image->isMask ? 'Yes' : 'No') . "\n";
            echo "  Description: " . ($image->description ?? 'N/A') . "\n";
            echo "  Data size: " . strlen($image->data) . " bytes\n";

            if ($image->ocrResult !== null) {
                echo "  OCR text length: " . strlen($image->ocrResult->content) . " characters\n";
            }

            echo "\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Page-Level Metadata ===\n\n";

try {
    $config = new ExtractionConfig(
        page: new PageConfig(
            extractPages: true,
            insertPageMarkers: false,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Page-Level Extraction:\n";
    echo "  Total pages: " . count($result->pages ?? []) . "\n\n";

    if ($result->pages !== null) {
        foreach (array_slice($result->pages, 0, 3) as $page) {
            echo "Page {$page->pageNumber} Metadata:\n";
            echo "  Content length: " . strlen($page->content) . " characters\n";
            echo "  Tables: " . count($page->tables) . "\n";
            echo "  Images: " . count($page->images) . "\n";

            if (!empty($page->tables)) {
                echo "  Table pages: ";
                echo implode(', ', array_map(
                    static fn ($t) => $t->pageNumber,
                    $page->tables
                ));
                echo "\n";
            }

            echo "\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Language Detection Metadata ===\n\n";

try {
    $config = new ExtractionConfig(
        languageDetection: new LanguageDetectionConfig(
            enabled: true,
            maxLanguages: 3,
            confidenceThreshold: 0.8,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/multilingual.pdf');

    echo "Language Detection:\n";

    if ($result->detectedLanguages !== null) {
        echo "  Detected languages: " . implode(', ', $result->detectedLanguages) . "\n";
    }

    echo "  Primary language (metadata): " . ($result->metadata->language ?? 'N/A') . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 8: Keyword Extraction Metadata ===\n\n";

try {
    $config = new ExtractionConfig(
        keyword: new KeywordConfig(
            maxKeywords: 10,
            minScore: 0.0,
            language: 'en',
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

    echo "Keyword Extraction:\n";

    if ($result->metadata->keywords !== null) {
        echo "  Extracted keywords:\n";
        foreach ($result->metadata->keywords as $i => $keyword) {
            echo "    " . ($i + 1) . ". {$keyword}\n";
        }
    } else {
        echo "  No keywords extracted\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 9: Comprehensive Metadata Report ===\n\n";

try {
    $config = new ExtractionConfig(
        imageExtraction: new ImageExtractionConfig(extractImages: true),
        page: new PageConfig(extractPages: true),
        languageDetection: new LanguageDetectionConfig(enabled: true),
        keyword: new KeywordConfig(maxKeywords: 5, minScore: 0.0, language: 'en'),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    echo "Comprehensive Metadata Report\n";
    echo str_repeat('=', 50) . "\n\n";

    echo "DOCUMENT INFORMATION\n";
    echo "  Title: " . ($result->metadata->title ?? 'N/A') . "\n";
    echo "  Subject: " . ($result->metadata->subject ?? 'N/A') . "\n";
    echo "  Authors: " . (
        $result->metadata->authors
            ? implode(', ', $result->metadata->authors)
            : 'N/A'
    ) . "\n";
    echo "  Created: " . ($result->metadata->createdAt ?? 'N/A') . "\n";
    echo "  Modified: " . ($result->metadata->modifiedAt ?? 'N/A') . "\n";
    echo "  Creator: " . ($result->metadata->createdBy ?? 'N/A') . "\n";
    echo "  Producer: " . ($result->metadata->producer ?? 'N/A') . "\n";

    echo "\nDOCUMENT STRUCTURE\n";
    echo "  MIME Type: {$result->mimeType}\n";
    echo "  Format: " . ($result->metadata->formatType ?? 'N/A') . "\n";
    echo "  Pages: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
    echo "  Content Length: " . number_format(strlen($result->content)) . " characters\n";

    echo "\nCONTENT ANALYSIS\n";
    echo "  Language: " . ($result->metadata->language ?? 'N/A') . "\n";
    if ($result->detectedLanguages !== null) {
        echo "  Detected Languages: " . implode(', ', $result->detectedLanguages) . "\n";
    }
    if ($result->metadata->keywords !== null) {
        echo "  Keywords: " . implode(', ', array_slice($result->metadata->keywords, 0, 5)) . "\n";
    }

    echo "\nSTRUCTURED DATA\n";
    echo "  Tables: " . count($result->tables) . "\n";
    echo "  Images: " . count($result->images ?? []) . "\n";
    echo "  Pages extracted: " . count($result->pages ?? []) . "\n";
    echo "  Chunks: " . count($result->chunks ?? []) . "\n";

    if (!empty($result->tables)) {
        echo "\n  Table Summary:\n";
        foreach ($result->tables as $i => $table) {
            echo "    Table " . ($i + 1) . ": Page {$table->pageNumber}, ";
            echo count($table->cells) . " rows\n";
        }
    }

    if (!empty($result->images)) {
        echo "\n  Image Summary:\n";
        foreach (array_slice($result->images, 0, 3) as $i => $image) {
            echo "    Image " . ($i + 1) . ": {$image->format}, ";
            echo "{$image->width}x{$image->height}px\n";
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 10: Metadata Comparison Across File Types ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/document.pdf' => 'PDF',
        __DIR__ . '/../sample-documents/spreadsheet.xlsx' => 'Excel',
        __DIR__ . '/../sample-documents/presentation.pptx' => 'PowerPoint',
        __DIR__ . '/../sample-documents/article.docx' => 'Word',
    ];

    $kreuzberg = new Kreuzberg();

    foreach ($files as $file => $type) {
        try {
            $result = $kreuzberg->extractFile($file);

            echo "{$type} Document:\n";
            echo "  Title: " . ($result->metadata->title ?? 'N/A') . "\n";
            echo "  Pages: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
            echo "  Format: {$result->mimeType}\n";
            echo "  Content: " . number_format(strlen($result->content)) . " chars\n";
            echo "  Tables: " . count($result->tables) . "\n";
            echo "\n";

        } catch (KreuzbergException $e) {
            echo "{$type}: Error - {$e->getMessage()}\n\n";
        }
    }

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 11: Procedural API for Metadata ===\n\n";

try {
    $result = extract_file(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Metadata (Procedural API):\n";
    echo "  Title: " . ($result->metadata->title ?? 'N/A') . "\n";
    echo "  Pages: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
    echo "  MIME: {$result->mimeType}\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 12: Exporting Metadata to JSON ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/document.pdf');

    $metadataArray = [
        'title' => $result->metadata->title,
        'subject' => $result->metadata->subject,
        'authors' => $result->metadata->authors,
        'language' => $result->metadata->language,
        'date' => $result->metadata->date,
        'created_at' => $result->metadata->createdAt,
        'modified_at' => $result->metadata->modifiedAt,
        'created_by' => $result->metadata->createdBy,
        'producer' => $result->metadata->producer,
        'page_count' => $result->metadata->pageCount,
        'format_type' => $result->metadata->formatType,
        'keywords' => $result->metadata->keywords,
        'custom' => $result->metadata->custom,
        'mime_type' => $result->mimeType,
        'content_length' => strlen($result->content),
        'table_count' => count($result->tables),
        'image_count' => count($result->images ?? []),
        'page_count_extracted' => count($result->pages ?? []),
    ];

    $json = json_encode($metadataArray, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE);

    echo "Metadata as JSON:\n";
    echo $json . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "Done!\n";
