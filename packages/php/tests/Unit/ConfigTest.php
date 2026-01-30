<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\PostProcessorConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Config\TokenReductionConfig;
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
#[CoversClass(ImageExtractionConfig::class)]
#[CoversClass(ImagePreprocessingConfig::class)]
#[CoversClass(KeywordConfig::class)]
#[CoversClass(LanguageDetectionConfig::class)]
#[CoversClass(PageConfig::class)]
#[CoversClass(TesseractConfig::class)]
#[CoversClass(PostProcessorConfig::class)]
#[CoversClass(TokenReductionConfig::class)]
final class ConfigTest extends TestCase
{
    #[Test]
    public function it_creates_default_extraction_config(): void
    {
        $config = new ExtractionConfig();

        $this->assertTrue($config->useCache);
        $this->assertTrue($config->enableQualityProcessing);
        $this->assertNull($config->ocr);
        $this->assertFalse($config->forceOcr);
        $this->assertNull($config->chunking);
        $this->assertNull($config->images);
        $this->assertNull($config->pdfOptions);
        $this->assertNull($config->tokenReduction);
        $this->assertNull($config->languageDetection);
        $this->assertNull($config->pages);
        $this->assertNull($config->keywords);
        $this->assertNull($config->postprocessor);
        $this->assertNull($config->htmlOptions);
        $this->assertNull($config->maxConcurrentExtractions);
        $this->assertSame('unified', $config->resultFormat);
        $this->assertSame('plain', $config->outputFormat);
    }

    #[Test]
    public function it_creates_extraction_config_with_custom_values(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            forceOcr: true,
            resultFormat: 'element_based',
            outputFormat: 'markdown',
            maxConcurrentExtractions: 5,
        );

        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertTrue($config->forceOcr);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
        $this->assertSame(5, $config->maxConcurrentExtractions);
    }

    #[Test]
    public function it_converts_extraction_config_to_array(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            forceOcr: true,
            resultFormat: 'element_based',
            outputFormat: 'markdown',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertFalse($array['use_cache']);
        $this->assertTrue($array['force_ocr']);
        $this->assertSame('element_based', $array['result_format']);
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
            extractMetadata: false,
            passwords: ['test'],
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_images']);
        $this->assertFalse($array['extract_metadata']);
        $this->assertSame(['test'], $array['passwords']);
    }

    #[Test]
    public function it_creates_chunking_config(): void
    {
        $config = new ChunkingConfig(
            maxChars: 1000,
            maxOverlap: 200,
            respectSentences: true,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame(1000, $array['max_chars']);
        $this->assertSame(200, $array['max_overlap']);
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
        $chunkingConfig = new ChunkingConfig(maxChars: 500);

        $config = new ExtractionConfig(
            ocr: $ocrConfig,
            pdfOptions: $pdfConfig,
            chunking: $chunkingConfig,
            forceOcr: true,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertArrayHasKey('ocr', $array);
        $this->assertArrayHasKey('pdf_options', $array);
        $this->assertArrayHasKey('chunking', $array);
        $this->assertTrue($array['force_ocr']);
        $this->assertSame('tesseract', $array['ocr']['backend']);
        $this->assertSame('eng', $array['ocr']['language']);
    }

    #[Test]
    public function it_filters_null_and_default_values_in_array_conversion(): void
    {
        $config = new ExtractionConfig();

        $array = $config->toArray();

        // Default values should be filtered out
        $this->assertArrayNotHasKey('use_cache', $array);
        $this->assertArrayNotHasKey('enable_quality_processing', $array);
        $this->assertArrayNotHasKey('force_ocr', $array);
        $this->assertArrayNotHasKey('result_format', $array);
        $this->assertArrayNotHasKey('output_format', $array);
        $this->assertArrayNotHasKey('ocr', $array);
        $this->assertArrayNotHasKey('pdf_options', $array);
        $this->assertArrayNotHasKey('chunking', $array);
        $this->assertArrayNotHasKey('images', $array);
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
        $config = new ExtractionConfig(forceOcr: true);

        $this->assertTrue($config->forceOcr);

        $reflection = new \ReflectionClass($config);
        $this->assertTrue($reflection->isReadOnly());
    }

    #[Test]
    public function it_creates_image_extraction_config(): void
    {
        $imageExtractionConfig = new ImageExtractionConfig(
            extractImages: true,
            targetDpi: 200,
            maxImageDimension: 3000,
        );

        $this->assertTrue($imageExtractionConfig->extractImages);
        $this->assertSame(200, $imageExtractionConfig->targetDpi);
        $this->assertSame(3000, $imageExtractionConfig->maxImageDimension);
    }

    #[Test]
    public function it_converts_extraction_config_with_html_options(): void
    {
        $htmlOptions = [
            'heading_style' => 'atx',
            'list_format' => 'unordered',
        ];

        $config = new ExtractionConfig(htmlOptions: $htmlOptions);

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertArrayHasKey('html_options', $array);
        $this->assertSame('atx', $array['html_options']['heading_style']);
        $this->assertSame('unordered', $array['html_options']['list_format']);
    }

    #[Test]
    public function it_creates_extraction_config_from_array(): void
    {
        $data = [
            'use_cache' => false,
            'enable_quality_processing' => false,
            'force_ocr' => true,
            'result_format' => 'element_based',
            'output_format' => 'markdown',
            'max_concurrent_extractions' => 4,
            'ocr' => [
                'backend' => 'tesseract',
                'language' => 'eng',
            ],
        ];

        $config = ExtractionConfig::fromArray($data);

        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertTrue($config->forceOcr);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
        $this->assertSame(4, $config->maxConcurrentExtractions);
        $this->assertNotNull($config->ocr);
        $this->assertSame('tesseract', $config->ocr->backend);
        $this->assertSame('eng', $config->ocr->language);
    }
}
