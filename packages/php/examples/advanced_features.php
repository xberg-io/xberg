<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use function Kreuzberg\extract_file;

echo "=== Text Chunking & Embeddings ===\n";

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 512,
        chunkOverlap: 50,
        respectSentences: true
    ),
    embedding: new EmbeddingConfig(
        model: 'all-MiniLM-L6-v2',
        normalize: true
    )
);

$result = extract_file(__DIR__ . '/document.pdf', config: $config);

if ($result->chunks !== null) {
    echo "Total chunks: " . count($result->chunks) . "\n";

    foreach (array_slice($result->chunks, 0, 3) as $chunk) {
        echo "\nChunk {$chunk->metadata->chunkIndex}:\n";
        echo "Length: " . strlen($chunk->content) . " characters\n";
        echo "Tokens: {$chunk->metadata->tokenCount}\n";

        if ($chunk->embedding !== null) {
            echo "Embedding dimension: " . count($chunk->embedding) . "\n";
            echo "First 5 values: " . implode(', ', array_slice($chunk->embedding, 0, 5)) . "...\n";
        }
    }
}

echo "\n";

echo "=== Image Extraction ===\n";

$config = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        performOcr: true,
        minWidth: 100,
        minHeight: 100
    ),
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

$result = extract_file(__DIR__ . '/presentation.pptx', config: $config);

if ($result->images !== null) {
    echo "Total images: " . count($result->images) . "\n";

    foreach ($result->images as $image) {
        echo "\nImage {$image->imageIndex}:\n";
        echo "Format: {$image->format}\n";
        echo "Size: {$image->width}x{$image->height}\n";
        echo "Page: {$image->pageNumber}\n";

        if ($image->ocrResult !== null) {
            echo "OCR Text: " . substr($image->ocrResult->content, 0, 100) . "...\n";
        }
    }
}

echo "\n";

echo "=== Page Extraction ===\n";

$config = new ExtractionConfig(
    page: new PageConfig(
        extractPages: true,
        insertPageMarkers: true,
        markerFormat: '--- Page {page_number} ---'
    )
);

$result = extract_file(__DIR__ . '/report.pdf', config: $config);

if ($result->pages !== null) {
    echo "Total pages: " . count($result->pages) . "\n";

    foreach (array_slice($result->pages, 0, 3) as $page) {
        echo "\n=== Page {$page->pageNumber} ===\n";
        echo "Content length: " . strlen($page->content) . " characters\n";
        echo "Tables: " . count($page->tables) . "\n";
        echo "Images: " . count($page->images) . "\n";
        echo "First 100 characters: " . substr($page->content, 0, 100) . "...\n";
    }
}

echo "\n";

echo "=== Language Detection & Keywords ===\n";

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        maxLanguages: 3,
        confidenceThreshold: 0.8
    ),
    keyword: new KeywordConfig(
        maxKeywords: 10,
        minScore: 0.0,
        language: 'en'
    )
);

$result = extract_file(__DIR__ . '/article.pdf', config: $config);

if ($result->detectedLanguages !== null) {
    echo "Detected languages: " . implode(', ', $result->detectedLanguages) . "\n";
}

if ($result->metadata->keywords !== null) {
    echo "Keywords: " . implode(', ', $result->metadata->keywords) . "\n";
}

echo "\n";
