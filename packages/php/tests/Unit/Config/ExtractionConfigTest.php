<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit\Config;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\HtmlConversionOptions;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\PostProcessorConfig;
use Kreuzberg\Config\TokenReductionConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for ExtractionConfig readonly class.
 *
 * Tests construction, serialization, factory methods, readonly enforcement,
 * and handling of complex nested configuration objects and boolean properties.
 * This is the main configuration class that aggregates all extraction settings.
 *
 * Test Coverage:
 * - Construction with default values
 * - Construction with custom values
 * - toArray() serialization with optional field inclusion
 * - fromArray() factory method with nested structures
 * - fromJson() factory method
 * - toJson() serialization
 * - Readonly enforcement
 * - Nested configuration handling
 * - Builder pattern
 * - Invalid JSON handling
 * - Round-trip serialization
 * - New fields: useCache, enableQualityProcessing, forceOcr, maxConcurrentExtractions, resultFormat, outputFormat
 */
#[CoversClass(ExtractionConfig::class)]
#[Group('unit')]
#[Group('config')]
final class ExtractionConfigTest extends TestCase
{
    #[Test]
    public function it_creates_with_default_values(): void
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
    public function it_creates_with_custom_values(): void
    {
        $ocrConfig = new OcrConfig(backend: 'tesseract');
        $pdfConfig = new PdfConfig(extractImages: true);
        $chunkingConfig = new ChunkingConfig(maxChars: 1024);
        $htmlOptions = HtmlConversionOptions::fromArray(['heading_style' => 'atx', 'code_block_style' => 'fenced']);

        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            ocr: $ocrConfig,
            forceOcr: true,
            chunking: $chunkingConfig,
            pdfOptions: $pdfConfig,
            maxConcurrentExtractions: 8,
            resultFormat: 'element_based',
            outputFormat: 'markdown',
            htmlOptions: $htmlOptions,
        );

        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertSame($ocrConfig, $config->ocr);
        $this->assertTrue($config->forceOcr);
        $this->assertSame($chunkingConfig, $config->chunking);
        $this->assertSame($pdfConfig, $config->pdfOptions);
        $this->assertSame(8, $config->maxConcurrentExtractions);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
        $this->assertInstanceOf(HtmlConversionOptions::class, $config->htmlOptions);
        $this->assertSame('atx', $config->htmlOptions->headingStyle);
        $this->assertSame('fenced', $config->htmlOptions->codeBlockStyle);
    }

    #[Test]
    public function it_serializes_to_array_with_only_non_default_values(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            forceOcr: true,
        );
        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertFalse($array['use_cache']);
        $this->assertFalse($array['enable_quality_processing']);
        $this->assertTrue($array['force_ocr']);
        $this->assertArrayNotHasKey('ocr', $array);
        $this->assertArrayNotHasKey('pdf_options', $array);
    }

    #[Test]
    public function it_includes_nested_configs_in_array_when_set(): void
    {
        $ocr = new OcrConfig();
        $pdf = new PdfConfig();
        $chunking = new ChunkingConfig();

        $config = new ExtractionConfig(
            ocr: $ocr,
            pdfOptions: $pdf,
            chunking: $chunking,
        );
        $array = $config->toArray();

        $this->assertArrayHasKey('ocr', $array);
        $this->assertArrayHasKey('pdf_options', $array);
        $this->assertArrayHasKey('chunking', $array);
        $this->assertIsArray($array['ocr']);
        $this->assertIsArray($array['pdf_options']);
        $this->assertIsArray($array['chunking']);
    }

    #[Test]
    public function it_creates_from_array_with_defaults(): void
    {
        $config = ExtractionConfig::fromArray([]);

        $this->assertTrue($config->useCache);
        $this->assertTrue($config->enableQualityProcessing);
        $this->assertNull($config->ocr);
        $this->assertFalse($config->forceOcr);
        $this->assertNull($config->maxConcurrentExtractions);
        $this->assertSame('unified', $config->resultFormat);
        $this->assertSame('plain', $config->outputFormat);
        $this->assertNull($config->htmlOptions);
    }

    #[Test]
    public function it_creates_from_array_with_all_fields(): void
    {
        $data = [
            'use_cache' => false,
            'enable_quality_processing' => false,
            'ocr' => ['backend' => 'tesseract', 'language' => 'eng'],
            'force_ocr' => true,
            'pdf_options' => ['extract_images' => true],
            'chunking' => ['max_chunk_size' => 512],
            'images' => ['extract_images' => true],
            'pages' => ['extract_pages' => true],
            'language_detection' => ['enabled' => true],
            'keywords' => ['max_keywords' => 10],
            'max_concurrent_extractions' => 16,
            'result_format' => 'element_based',
            'output_format' => 'markdown',
            'html_options' => ['heading_style' => 'setext', 'list_style' => 'dash'],
        ];
        $config = ExtractionConfig::fromArray($data);

        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertNotNull($config->ocr);
        $this->assertTrue($config->forceOcr);
        $this->assertNotNull($config->pdfOptions);
        $this->assertNotNull($config->chunking);
        $this->assertNotNull($config->images);
        $this->assertNotNull($config->pages);
        $this->assertNotNull($config->languageDetection);
        $this->assertNotNull($config->keywords);
        $this->assertSame(16, $config->maxConcurrentExtractions);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
        $this->assertInstanceOf(HtmlConversionOptions::class, $config->htmlOptions);
        $this->assertSame('setext', $config->htmlOptions->headingStyle);
        $this->assertSame('dash', $config->htmlOptions->listStyle);
    }

    #[Test]
    public function it_serializes_to_json(): void
    {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract'),
            forceOcr: true,
            outputFormat: 'markdown',
            useCache: false,
            maxConcurrentExtractions: 6,
        );
        $json = $config->toJson();

        $this->assertJson($json);
        $decoded = json_decode($json, true);

        $this->assertArrayHasKey('ocr', $decoded);
        $this->assertTrue($decoded['force_ocr']);
        $this->assertSame('markdown', $decoded['output_format']);
        $this->assertFalse($decoded['use_cache']);
        $this->assertSame(6, $decoded['max_concurrent_extractions']);
    }

    #[Test]
    public function it_creates_from_json(): void
    {
        $json = json_encode([
            'ocr' => ['backend' => 'easyocr'],
            'use_cache' => false,
            'force_ocr' => true,
            'max_concurrent_extractions' => 12,
            'result_format' => 'element_based',
        ]);
        $config = ExtractionConfig::fromJson($json);

        $this->assertNotNull($config->ocr);
        $this->assertFalse($config->useCache);
        $this->assertTrue($config->forceOcr);
        $this->assertSame(12, $config->maxConcurrentExtractions);
        $this->assertSame('element_based', $config->resultFormat);
    }

    #[Test]
    public function it_round_trips_through_json(): void
    {
        $htmlOptions = HtmlConversionOptions::fromArray(['heading_style' => 'atx', 'code_block_style' => 'fenced']);
        $original = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
            forceOcr: true,
            pdfOptions: new PdfConfig(extractImages: true),
            chunking: new ChunkingConfig(maxChars: 1024),
            maxConcurrentExtractions: 8,
            resultFormat: 'element_based',
            outputFormat: 'markdown',
            htmlOptions: $htmlOptions,
        );

        $json = $original->toJson();
        $restored = ExtractionConfig::fromJson($json);

        $this->assertNotNull($restored->ocr);
        $this->assertNotNull($restored->pdfOptions);
        $this->assertNotNull($restored->chunking);
        $this->assertSame($original->useCache, $restored->useCache);
        $this->assertSame($original->enableQualityProcessing, $restored->enableQualityProcessing);
        $this->assertSame($original->forceOcr, $restored->forceOcr);
        $this->assertSame($original->maxConcurrentExtractions, $restored->maxConcurrentExtractions);
        $this->assertSame($original->resultFormat, $restored->resultFormat);
        $this->assertSame($original->outputFormat, $restored->outputFormat);
        $this->assertEquals($original->htmlOptions, $restored->htmlOptions);
    }

    #[Test]
    public function it_throws_on_invalid_json(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('Invalid JSON');

        ExtractionConfig::fromJson('{ invalid }');
    }

    #[Test]
    public function it_enforces_readonly_on_use_cache_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ExtractionConfig(useCache: true);
        $config->useCache = false;
    }

    #[Test]
    public function it_enforces_readonly_on_output_format_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ExtractionConfig(outputFormat: 'markdown');
        $config->outputFormat = 'plain';
    }

    #[Test]
    public function it_enforces_readonly_on_ocr_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ExtractionConfig(ocr: new OcrConfig());
        $config->ocr = new OcrConfig(backend: 'easyocr');
    }

    #[Test]
    public function it_enforces_readonly_on_max_concurrent_extractions_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ExtractionConfig(maxConcurrentExtractions: 8);
        $config->maxConcurrentExtractions = 4;
    }

    #[Test]
    public function it_enforces_readonly_on_html_options_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ExtractionConfig(htmlOptions: HtmlConversionOptions::fromArray(['heading_style' => 'atx']));
        $config->htmlOptions = HtmlConversionOptions::fromArray(['heading_style' => 'setext']);
    }

    #[Test]
    public function it_creates_from_file(): void
    {
        $tempFile = tempnam(sys_get_temp_dir(), 'extract_');
        if ($tempFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tempFile, json_encode([
                'use_cache' => false,
                'force_ocr' => true,
                'ocr' => ['backend' => 'tesseract'],
                'max_concurrent_extractions' => 10,
            ]));

            $config = ExtractionConfig::fromFile($tempFile);

            $this->assertFalse($config->useCache);
            $this->assertTrue($config->forceOcr);
            $this->assertNotNull($config->ocr);
            $this->assertSame(10, $config->maxConcurrentExtractions);
        } finally {
            if (file_exists($tempFile)) {
                unlink($tempFile);
            }
        }
    }

    #[Test]
    public function it_throws_when_file_not_found(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('File not found');

        ExtractionConfig::fromFile('/nonexistent/path/config.json');
    }

    #[Test]
    public function it_handles_type_coercion_for_use_cache(): void
    {
        $data = ['use_cache' => 0];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsBool($config->useCache);
        $this->assertFalse($config->useCache);
    }

    #[Test]
    public function it_handles_type_coercion_for_enable_quality_processing(): void
    {
        $data = ['enable_quality_processing' => 0];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsBool($config->enableQualityProcessing);
        $this->assertFalse($config->enableQualityProcessing);
    }

    #[Test]
    public function it_handles_type_coercion_for_force_ocr(): void
    {
        $data = ['force_ocr' => 'true'];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsBool($config->forceOcr);
        $this->assertTrue($config->forceOcr);
    }

    #[Test]
    public function it_handles_type_coercion_for_max_concurrent_extractions(): void
    {
        $data = ['max_concurrent_extractions' => '8'];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsInt($config->maxConcurrentExtractions);
        $this->assertSame(8, $config->maxConcurrentExtractions);
    }

    #[Test]
    public function it_handles_type_coercion_for_result_format(): void
    {
        $data = ['result_format' => 123];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsString($config->resultFormat);
        $this->assertSame('123', $config->resultFormat);
    }

    #[Test]
    public function it_handles_type_coercion_for_output_format(): void
    {
        $data = ['output_format' => 456];
        $config = ExtractionConfig::fromArray($data);

        $this->assertIsString($config->outputFormat);
        $this->assertSame('456', $config->outputFormat);
    }

    #[Test]
    public function it_has_builder_method(): void
    {
        $this->assertTrue(method_exists(ExtractionConfig::class, 'builder'));
    }

    #[Test]
    public function it_supports_builder_with_new_fields(): void
    {
        $config = ExtractionConfig::builder()
            ->withUseCache(false)
            ->withEnableQualityProcessing(false)
            ->withForceOcr(true)
            ->withMaxConcurrentExtractions(12)
            ->withResultFormat('element_based')
            ->withOutputFormat('markdown')
            ->build();

        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertTrue($config->forceOcr);
        $this->assertSame(12, $config->maxConcurrentExtractions);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
    }

    #[Test]
    public function it_supports_all_nested_configs_together(): void
    {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(),
            pdfOptions: new PdfConfig(),
            chunking: new ChunkingConfig(),
            images: new ImageExtractionConfig(),
            pages: new PageConfig(),
            languageDetection: new LanguageDetectionConfig(),
            keywords: new KeywordConfig(),
        );

        $array = $config->toArray();

        $this->assertArrayHasKey('ocr', $array);
        $this->assertArrayHasKey('pdf_options', $array);
        $this->assertArrayHasKey('chunking', $array);
        $this->assertArrayHasKey('images', $array);
        $this->assertArrayHasKey('pages', $array);
        $this->assertArrayHasKey('language_detection', $array);
        $this->assertArrayHasKey('keywords', $array);
    }

    #[Test]
    public function it_json_output_is_prettified(): void
    {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(),
            forceOcr: true,
        );
        $json = $config->toJson();

        $this->assertStringContainsString("\n", $json);
        $this->assertStringContainsString('  ', $json);
    }

    #[Test]
    public function it_serializes_non_default_values_for_new_fields(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            forceOcr: true,
            maxConcurrentExtractions: 10,
            resultFormat: 'element_based',
            outputFormat: 'markdown',
        );
        $array = $config->toArray();

        $this->assertFalse($array['use_cache']);
        $this->assertFalse($array['enable_quality_processing']);
        $this->assertTrue($array['force_ocr']);
        $this->assertSame(10, $array['max_concurrent_extractions']);
        $this->assertSame('element_based', $array['result_format']);
        $this->assertSame('markdown', $array['output_format']);
    }

    #[Test]
    public function it_omits_default_values_for_new_fields_in_serialization(): void
    {
        $config = new ExtractionConfig(
            useCache: true,
            enableQualityProcessing: true,
            forceOcr: false,
            maxConcurrentExtractions: null,
            resultFormat: 'unified',
            outputFormat: 'plain',
        );
        $array = $config->toArray();

        $this->assertArrayNotHasKey('use_cache', $array);
        $this->assertArrayNotHasKey('enable_quality_processing', $array);
        $this->assertArrayNotHasKey('force_ocr', $array);
        $this->assertArrayNotHasKey('max_concurrent_extractions', $array);
        $this->assertArrayNotHasKey('result_format', $array);
        $this->assertArrayNotHasKey('output_format', $array);
    }

    #[Test]
    public function it_allows_various_max_concurrent_extractions_values(): void
    {
        $values = [1, 2, 4, 8, 16, 32, 100];

        foreach ($values as $value) {
            $config = new ExtractionConfig(maxConcurrentExtractions: $value);
            $this->assertSame($value, $config->maxConcurrentExtractions);
        }
    }

    #[Test]
    public function it_allows_null_max_concurrent_extractions(): void
    {
        $config = new ExtractionConfig(maxConcurrentExtractions: null);
        $this->assertNull($config->maxConcurrentExtractions);
    }

    #[Test]
    public function it_allows_various_result_formats(): void
    {
        $formats = ['unified', 'element_based', 'custom'];

        foreach ($formats as $format) {
            $config = new ExtractionConfig(resultFormat: $format);
            $this->assertSame($format, $config->resultFormat);
        }
    }

    #[Test]
    public function it_allows_various_output_formats(): void
    {
        $formats = ['plain', 'markdown', 'djot', 'html'];

        foreach ($formats as $format) {
            $config = new ExtractionConfig(outputFormat: $format);
            $this->assertSame($format, $config->outputFormat);
        }
    }

    #[Test]
    public function it_handles_html_options_in_serialization(): void
    {
        $htmlOptions = new HtmlConversionOptions(
            headingStyle: 'atx',
            codeBlockStyle: 'fenced',
        );
        $config = new ExtractionConfig(htmlOptions: $htmlOptions);
        $array = $config->toArray();

        $this->assertArrayHasKey('html_options', $array);
        $this->assertSame($htmlOptions->toArray(), $array['html_options']);
    }

    #[Test]
    public function it_handles_html_options_in_deserialization(): void
    {
        $data = [
            'html_options' => [
                'heading_style' => 'setext',
                'code_block_style' => 'indented',
            ],
        ];
        $config = ExtractionConfig::fromArray($data);

        $this->assertInstanceOf(HtmlConversionOptions::class, $config->htmlOptions);
        $this->assertSame('setext', $config->htmlOptions->headingStyle);
        $this->assertSame('indented', $config->htmlOptions->codeBlockStyle);
    }

    #[Test]
    public function it_omits_null_html_options_from_serialization(): void
    {
        $config = new ExtractionConfig(htmlOptions: null);
        $array = $config->toArray();

        $this->assertArrayNotHasKey('html_options', $array);
    }

    #[Test]
    public function it_handles_empty_html_options_array(): void
    {
        $config = new ExtractionConfig(htmlOptions: new HtmlConversionOptions());
        $array = $config->toArray();

        // Empty HtmlConversionOptions (all nulls) serializes to empty array
        $this->assertArrayHasKey('html_options', $array);
        $this->assertSame([], $array['html_options']);
    }

    #[Test]
    public function it_provides_complete_builder_chain_with_all_new_fields(): void
    {
        $htmlOptions = ['heading_style' => 'atx'];
        $config = ExtractionConfig::builder()
            ->withOcr(new OcrConfig())
            ->withUseCache(false)
            ->withEnableQualityProcessing(false)
            ->withForceOcr(true)
            ->withMaxConcurrentExtractions(16)
            ->withResultFormat('element_based')
            ->withOutputFormat('markdown')
            ->withHtmlOptions($htmlOptions)
            ->build();

        $this->assertNotNull($config->ocr);
        $this->assertFalse($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertTrue($config->forceOcr);
        $this->assertSame(16, $config->maxConcurrentExtractions);
        $this->assertSame('element_based', $config->resultFormat);
        $this->assertSame('markdown', $config->outputFormat);
        $this->assertInstanceOf(HtmlConversionOptions::class, $config->htmlOptions);
        $this->assertSame('atx', $config->htmlOptions->headingStyle);
    }

    #[Test]
    public function it_builder_has_correct_defaults(): void
    {
        $config = ExtractionConfig::builder()->build();

        $this->assertTrue($config->useCache);
        $this->assertTrue($config->enableQualityProcessing);
        $this->assertNull($config->ocr);
        $this->assertFalse($config->forceOcr);
        $this->assertNull($config->chunking);
        $this->assertNull($config->images);
        $this->assertNull($config->pdfOptions);
        $this->assertNull($config->maxConcurrentExtractions);
        $this->assertSame('unified', $config->resultFormat);
        $this->assertSame('plain', $config->outputFormat);
    }

    #[Test]
    public function it_supports_postprocessor_config(): void
    {
        $postprocessor = new PostProcessorConfig();
        $config = new ExtractionConfig(postprocessor: $postprocessor);

        $this->assertSame($postprocessor, $config->postprocessor);
        $array = $config->toArray();
        $this->assertArrayHasKey('postprocessor', $array);
    }

    #[Test]
    public function it_supports_token_reduction_config(): void
    {
        $tokenReduction = new TokenReductionConfig();
        $config = new ExtractionConfig(tokenReduction: $tokenReduction);

        $this->assertSame($tokenReduction, $config->tokenReduction);
        $array = $config->toArray();
        $this->assertArrayHasKey('token_reduction', $array);
    }

    #[Test]
    public function it_builder_supports_all_config_types(): void
    {
        $config = ExtractionConfig::builder()
            ->withOcr(new OcrConfig())
            ->withPdfOptions(new PdfConfig())
            ->withChunking(new ChunkingConfig())
            ->withImages(new ImageExtractionConfig())
            ->withPages(new PageConfig())
            ->withLanguageDetection(new LanguageDetectionConfig())
            ->withKeywords(new KeywordConfig())
            ->withPostprocessor(new PostProcessorConfig())
            ->withTokenReduction(new TokenReductionConfig())
            ->build();

        $this->assertNotNull($config->ocr);
        $this->assertNotNull($config->pdfOptions);
        $this->assertNotNull($config->chunking);
        $this->assertNotNull($config->images);
        $this->assertNotNull($config->pages);
        $this->assertNotNull($config->languageDetection);
        $this->assertNotNull($config->keywords);
        $this->assertNotNull($config->postprocessor);
        $this->assertNotNull($config->tokenReduction);
    }
}
