<?php

declare(strict_types=1);

/**
 * Advanced Configuration Example
 *
 * Demonstrates complex configurations with all available options.
 * Shows how to fine-tune extraction behavior for specific use cases.
 *
 * This example covers:
 * - PDF-specific configuration
 * - Image extraction configuration
 * - Page extraction with markers
 * - Language detection
 * - Keyword extraction
 * - Combining multiple configuration options
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;


echo "=== Example 1: PDF-Specific Configuration ===\n\n";

try {
    $config = new ExtractionConfig(
        pdf: new PdfConfig(
            extractImages: true,
            extractMetadata: true,
            ocrFallback: false,
            startPage: 1,
            endPage: 5,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Extracted pages 1-5:\n";
    echo "  Content length: " . strlen($result->content) . " characters\n";
    echo "  Pages: {$result->metadata->pageCount}\n";
    echo "  Images found: " . count($result->images ?? []) . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 2: Advanced OCR Configuration ===\n\n";

try {
    $config = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: 'eng+deu',
            tesseractConfig: new TesseractConfig(
                psm: 6,
                enableTableDetection: true,
            ),
        ),
        pdf: new PdfConfig(
            ocrFallback: true,
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/scanned.pdf');

    echo "OCR extraction complete:\n";
    echo "  Content length: " . strlen($result->content) . " characters\n";
    echo "  Tables found: " . count($result->tables) . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 3: Image Extraction with OCR ===\n\n";

try {
    $config = new ExtractionConfig(
        imageExtraction: new ImageExtractionConfig(
            extractImages: true,
            performOcr: true,
            minWidth: 100,
            minHeight: 100,
        ),
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: 'eng',
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/presentation.pptx');

    echo "Image extraction results:\n";
    echo "  Total images: " . count($result->images ?? []) . "\n";

    if ($result->images !== null) {
        foreach (array_slice($result->images, 0, 3) as $i => $image) {
            echo "\n  Image " . ($i + 1) . ":\n";
            echo "    Format: {$image->format}\n";
            echo "    Size: {$image->width}x{$image->height} pixels\n";
            echo "    Page: {$image->pageNumber}\n";

            if ($image->ocrResult !== null) {
                echo "    OCR text length: " . strlen($image->ocrResult->content) . " characters\n";
                echo "    First 100 chars: " . substr($image->ocrResult->content, 0, 100) . "...\n";
            }
        }
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 4: Page Extraction with Markers ===\n\n";

try {
    $config = new ExtractionConfig(
        page: new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
            markerFormat: '--- Page {page_number} ---',
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Page extraction results:\n";
    echo "  Total pages: " . count($result->pages ?? []) . "\n";

    if ($result->pages !== null) {
        foreach (array_slice($result->pages, 0, 2) as $page) {
            echo "\n=== Page {$page->pageNumber} ===\n";
            echo "Content length: " . strlen($page->content) . " characters\n";
            echo "Tables: " . count($page->tables) . "\n";
            echo "Images: " . count($page->images) . "\n";
            echo "\nFirst 200 characters:\n";
            echo substr($page->content, 0, 200) . "...\n";
        }
    }

    echo "\n--- Content with page markers ---\n";
    echo substr($result->content, 0, 500) . "...\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 5: Language Detection ===\n\n";

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

    echo "Language detection results:\n";

    if ($result->detectedLanguages !== null) {
        echo "  Detected languages: " . implode(', ', $result->detectedLanguages) . "\n";
    }

    echo "  Primary language: " . ($result->metadata->language ?? 'N/A') . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 6: Keyword Extraction ===\n\n";

try {
    $config = new ExtractionConfig(
        keyword: new KeywordConfig(
            maxKeywords: 10,
            minScore: 0.0,
            language: 'en'
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/article.pdf');

    echo "Keyword extraction results:\n";

    if ($result->metadata->keywords !== null) {
        echo "  Keywords: " . implode(', ', $result->metadata->keywords) . "\n";
    }

    echo "\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 7: Comprehensive Configuration (All Options) ===\n\n";

try {
    $config = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'tesseract',
            language: 'eng',
            tesseractConfig: new TesseractConfig(
                psm: 6,
                enableTableDetection: true,
            ),
        ),

        pdf: new PdfConfig(
            extractImages: true,
            extractMetadata: true,
            ocrFallback: true,
        ),

        chunking: new ChunkingConfig(
            maxChunkSize: 512,
            chunkOverlap: 50,
            respectSentences: true,
            respectParagraphs: true,
        ),

        embedding: new EmbeddingConfig(
            model: 'all-MiniLM-L6-v2',
            normalize: true,
            batchSize: 32,
        ),

        imageExtraction: new ImageExtractionConfig(
            extractImages: true,
            performOcr: true,
            minWidth: 100,
            minHeight: 100,
        ),

        page: new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
            markerFormat: '=== Page {page_number} ===',
        ),

        languageDetection: new LanguageDetectionConfig(
            enabled: true,
            maxLanguages: 3,
            confidenceThreshold: 0.8,
        ),

        keyword: new KeywordConfig(
            maxKeywords: 10,
            minScore: 0.0,
            language: 'en',
        ),

        extractImages: true,
        extractTables: true,
        preserveFormatting: false,
        outputFormat: 'markdown',
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/sample.pdf');

    echo "Comprehensive extraction results:\n";
    echo "  Content length: " . strlen($result->content) . " characters\n";
    echo "  MIME type: {$result->mimeType}\n";
    echo "  Tables: " . count($result->tables) . "\n";
    echo "  Images: " . count($result->images ?? []) . "\n";
    echo "  Pages: " . count($result->pages ?? []) . "\n";
    echo "  Chunks: " . count($result->chunks ?? []) . "\n";
    echo "  Detected languages: " . (
        $result->detectedLanguages
            ? implode(', ', $result->detectedLanguages)
            : 'N/A'
    ) . "\n";
    echo "  Keywords: " . (
        $result->metadata->keywords
            ? implode(', ', $result->metadata->keywords)
            : 'N/A'
    ) . "\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}


echo "=== Example 8: Dynamic Configuration Based on File Type ===\n\n";

try {
    $filePath = __DIR__ . '/../sample-documents/sample.pdf';

    $mimeType = \Kreuzberg\detect_mime_type_from_path($filePath);
    echo "Detected MIME type: {$mimeType}\n";

    $config = match (true) {
        str_contains($mimeType, 'pdf') => new ExtractionConfig(
            pdf: new PdfConfig(extractImages: true),
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
        ),
        str_contains($mimeType, 'image') => new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
        ),
        str_contains($mimeType, 'spreadsheet') => new ExtractionConfig(
            extractTables: true,
        ),
        default => new ExtractionConfig(),
    };

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($filePath);

    echo "Extracted with dynamic config:\n";
    echo "  Content length: " . strlen($result->content) . " characters\n\n";

} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n\n";
}

echo "Done!\n";
