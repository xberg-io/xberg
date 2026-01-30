<?php

declare(strict_types=1);

namespace Kreuzberg\Tests;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use PHPUnit\Framework\TestCase;

/**
 * Cross-language serialization tests for PHP bindings.
 *
 * Validates that ExtractionConfig serializes consistently with other language bindings.
 */
class SerializationTest extends TestCase
{
    /**
     * Test minimal config serialization.
     */
    public function testMinimalSerialization(): void
    {
        $config = new ExtractionConfig();
        $json = $config->toJson();

        $this->assertIsString($json);

        $parsed = json_decode($json, associative: true);
        // Default values should not be in serialization
        $this->assertIsArray($parsed);
    }

    /**
     * Test config serialization with custom values.
     */
    public function testCustomValuesSerialization(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: false,
            forceOcr: true,
        );

        $json = $config->toJson();
        $parsed = json_decode($json, associative: true);

        // useCache is non-default (false), so it should be in the output
        $this->assertEquals(false, $parsed['use_cache']);
        // enableQualityProcessing is non-default (false), so it should be in the output
        $this->assertEquals(false, $parsed['enable_quality_processing']);
        // forceOcr is non-default (true), so it should be in the output
        $this->assertEquals(true, $parsed['force_ocr']);
    }

    /**
     * Test field preservation after serialization.
     */
    public function testFieldPreservation(): void
    {
        $config = new ExtractionConfig(
            useCache: false,
            enableQualityProcessing: true,
        );

        $json = $config->toJson();
        $parsed = json_decode($json, associative: true);

        // useCache is non-default (false), so it should be in the output
        $this->assertArrayHasKey('use_cache', $parsed);
        $this->assertEquals(false, $parsed['use_cache']);
        // enableQualityProcessing is default (true), so it should not be in the output
        $this->assertArrayNotHasKey('enable_quality_processing', $parsed);
    }

    /**
     * Test round-trip serialization.
     */
    public function testRoundTripSerialization(): void
    {
        $config1 = new ExtractionConfig(
            useCache: true,
            enableQualityProcessing: false,
        );

        $json1 = $config1->toJson();
        $config2 = ExtractionConfig::fromJson($json1);

        // Verify round-trip preserves values
        $this->assertEquals($config1->useCache, $config2->useCache);
        $this->assertEquals($config1->enableQualityProcessing, $config2->enableQualityProcessing);
    }

    /**
     * Test snake_case field names.
     */
    public function testSnakeCaseFieldNames(): void
    {
        $config = new ExtractionConfig(useCache: false);
        $json = $config->toJson();

        $this->assertStringContainsString('use_cache', $json);
        $this->assertStringNotContainsString('useCache', $json);
    }

    /**
     * Test nested OCR config serialization.
     */
    public function testNestedOcrConfig(): void
    {
        $ocrConfig = new OcrConfig(
            backend: 'tesseract',
            language: 'eng',
        );

        $config = new ExtractionConfig(ocr: $ocrConfig);
        $json = json_encode($config);
        $parsed = json_decode($json, associative: true);

        $this->assertArrayHasKey('ocr', $parsed);
        $this->assertEquals('tesseract', $parsed['ocr']['backend']);
        $this->assertEquals('eng', $parsed['ocr']['language']);
    }

    /**
     * Test null value handling.
     */
    public function testNullValueHandling(): void
    {
        $config = new ExtractionConfig(
            ocr: null,
            chunking: null,
        );

        $json = json_encode($config);
        $parsed = json_decode($json, associative: true);

        // Should handle null values without errors
        $this->assertIsArray($parsed);
    }

    /**
     * Test immutability during serialization.
     */
    public function testImmutabilityDuringSerialization(): void
    {
        $config = new ExtractionConfig(useCache: true);

        $json1 = $config->toJson();
        $json2 = $config->toJson();
        $json3 = $config->toJson();

        $this->assertEquals($json1, $json2);
        $this->assertEquals($json2, $json3);
    }

    /**
     * Test optional fields omitted when they have default values.
     */
    public function testDefaultFieldsOmitted(): void
    {
        $config = new ExtractionConfig();
        $array = $config->toArray();

        // Default values should be omitted from serialization
        $this->assertArrayNotHasKey('use_cache', $array);
        $this->assertArrayNotHasKey('enable_quality_processing', $array);
        $this->assertArrayNotHasKey('force_ocr', $array);
    }

    /**
     * Test deserialization from JSON string.
     */
    public function testDeserialization(): void
    {
        $json = '{"use_cache":true,"enable_quality_processing":false,"force_ocr":true}';
        $config = ExtractionConfig::fromJson($json);

        $this->assertTrue($config->useCache);
        $this->assertFalse($config->enableQualityProcessing);
        $this->assertTrue($config->forceOcr);
    }

    /**
     * Test JSON encoding with pretty print.
     */
    public function testPrettyPrint(): void
    {
        $config = new ExtractionConfig(forceOcr: true);
        $json = $config->toJson();

        // Should have newlines (toJson uses JSON_PRETTY_PRINT)
        $this->assertStringContainsString("\n", $json);

        // Should still be valid JSON
        $parsed = json_decode($json, associative: true);
        $this->assertIsArray($parsed);
    }
}
