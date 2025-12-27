<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\TesseractConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for configuration classes.
 *
 * Tests the creation, validation, and serialization of all configuration
 * objects used in the Kreuzberg document extraction library.
 */
#[CoversClass(ExtractionConfig::class)]
#[CoversClass(OcrConfig::class)]
#[CoversClass(PdfConfig::class)]
#[CoversClass(ChunkingConfig::class)]
#[CoversClass(EmbeddingConfig::class)]
#[CoversClass(ImageExtractionConfig::class)]
#[CoversClass(ImagePreprocessingConfig::class)]
#[CoversClass(KeywordConfig::class)]
#[CoversClass(LanguageDetectionConfig::class)]
#[CoversClass(PageConfig::class)]
#[CoversClass(TesseractConfig::class)]
final class ConfigTest extends TestCase
{
    #[Test]
    public function it_creates_default_extraction_config(): void
    {
        $config = new ExtractionConfig();

        $this->assertNull($config->ocr);
        $this->assertNull($config->pdf);
        $this->assertNull($config->chunking);
        $this->assertNull($config->embedding);
        $this->assertNull($config->imageExtraction);
        $this->assertNull($config->page);
        $this->assertNull($config->languageDetection);
        $this->assertNull($config->keyword);
        $this->assertFalse($config->extractImages);
        $this->assertTrue($config->extractTables);
        $this->assertFalse($config->preserveFormatting);
        $this->assertNull($config->outputFormat);
    }

    #[Test]
    public function it_creates_extraction_config_with_custom_values(): void
    {
        $config = new ExtractionConfig(
            extractImages: true,
            extractTables: false,
            preserveFormatting: true,
            outputFormat: 'markdown',
        );

        $this->assertTrue($config->extractImages);
        $this->assertFalse($config->extractTables);
        $this->assertTrue($config->preserveFormatting);
        $this->assertSame('markdown', $config->outputFormat);
    }

    #[Test]
    public function it_converts_extraction_config_to_array(): void
    {
        $config = new ExtractionConfig(
            extractImages: true,
            extractTables: false,
            preserveFormatting: true,
            outputFormat: 'markdown',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_images']);
        $this->assertFalse($array['extract_tables']);
        $this->assertTrue($array['preserve_formatting']);
        $this->assertSame('markdown', $array['output_format']);
    }

    #[Test]
    public function it_creates_ocr_config(): void
    {
        $config = new OcrConfig(
            backend: 'tesseract',
            language: 'eng',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame('tesseract', $array['backend']);
        $this->assertSame('eng', $array['language']);
    }

    #[Test]
    public function it_creates_pdf_config(): void
    {
        $config = new PdfConfig(
            extractImages: true,
            extractMetadata: true,
            ocrFallback: false,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_images']);
        $this->assertTrue($array['extract_metadata']);
        $this->assertFalse($array['ocr_fallback']);
    }

    #[Test]
    public function it_creates_chunking_config(): void
    {
        $config = new ChunkingConfig(
            maxChunkSize: 1000,
            chunkOverlap: 200,
            respectSentences: true,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame(1000, $array['max_chunk_size']);
        $this->assertSame(200, $array['chunk_overlap']);
        $this->assertTrue($array['respect_sentences']);
    }

    #[Test]
    public function it_creates_tesseract_config(): void
    {
        $config = new TesseractConfig(
            psm: 3,
            oem: 3,
            enableTableDetection: true,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame(3, $array['psm']);
        $this->assertSame(3, $array['oem']);
        $this->assertTrue($array['enable_table_detection']);
    }

    #[Test]
    public function it_creates_nested_extraction_config(): void
    {
        $ocrConfig = new OcrConfig(backend: 'tesseract', language: 'eng');
        $pdfConfig = new PdfConfig(extractImages: true);
        $chunkingConfig = new ChunkingConfig(maxChunkSize: 500);

        $config = new ExtractionConfig(
            ocr: $ocrConfig,
            pdf: $pdfConfig,
            chunking: $chunkingConfig,
            extractImages: true,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertArrayHasKey('ocr', $array);
        $this->assertArrayHasKey('pdf', $array);
        $this->assertArrayHasKey('chunking', $array);
        $this->assertTrue($array['extract_images']);
        $this->assertSame('tesseract', $array['ocr']['backend']);
        $this->assertSame('eng', $array['ocr']['language']);
    }

    #[Test]
    public function it_filters_null_values_in_array_conversion(): void
    {
        $config = new ExtractionConfig(
            extractImages: false,
            extractTables: true,
        );

        $array = $config->toArray();

        $this->assertArrayNotHasKey('ocr', $array);
        $this->assertArrayNotHasKey('pdf', $array);
        $this->assertArrayNotHasKey('chunking', $array);
        $this->assertArrayNotHasKey('output_format', $array);
    }

    #[Test]
    public function it_creates_page_config(): void
    {
        $config = new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
            markerFormat: '--- Page {page_number} ---',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_pages']);
        $this->assertTrue($array['insert_page_markers']);
        $this->assertSame('--- Page {page_number} ---', $array['marker_format']);
    }

    #[Test]
    public function it_creates_language_detection_config(): void
    {
        $config = new LanguageDetectionConfig(
            enabled: true,
            confidenceThreshold: 0.8,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['enabled']);
        $this->assertSame(0.8, $array['confidence_threshold']);
    }

    #[Test]
    public function it_creates_keyword_config(): void
    {
        $config = new KeywordConfig(
            maxKeywords: 10,
            minScore: 0.5,
            language: 'en',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame(10, $array['max_keywords']);
        $this->assertSame(0.5, $array['min_score']);
        $this->assertSame('en', $array['language']);
    }

    #[Test]
    public function it_creates_readonly_config_objects(): void
    {
        $config = new ExtractionConfig(extractImages: true);

        $this->assertTrue($config->extractImages);

        $reflection = new \ReflectionClass($config);
        $this->assertTrue($reflection->isReadOnly());
    }
}
