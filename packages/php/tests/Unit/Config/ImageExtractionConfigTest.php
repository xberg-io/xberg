<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit\Config;

use Kreuzberg\Config\ImageExtractionConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for ImageExtractionConfig readonly class.
 *
 * Tests construction, serialization, factory methods, readonly enforcement,
 * and handling of boolean and nullable integer properties.
 *
 * Test Coverage:
 * - Construction with default values
 * - Construction with custom values
 * - toArray() serialization with optional field inclusion
 * - fromArray() factory method
 * - fromJson() factory method
 * - toJson() serialization
 * - Readonly enforcement
 * - Null handling
 * - Invalid JSON handling
 * - Round-trip serialization
 */
#[CoversClass(ImageExtractionConfig::class)]
#[Group('unit')]
#[Group('config')]
final class ImageExtractionConfigTest extends TestCase
{
    #[Test]
    public function it_creates_with_default_values(): void
    {
        $config = new ImageExtractionConfig();

        $this->assertTrue($config->extractImages);
        $this->assertSame(300, $config->targetDpi);
        $this->assertSame(4096, $config->maxImageDimension);
        $this->assertTrue($config->autoAdjustDpi);
        $this->assertSame(72, $config->minDpi);
        $this->assertSame(600, $config->maxDpi);
    }

    #[Test]
    public function it_creates_with_custom_values(): void
    {
        $config = new ImageExtractionConfig(
            extractImages: false,
            targetDpi: 150,
            maxImageDimension: 2048,
            autoAdjustDpi: false,
        );

        $this->assertFalse($config->extractImages);
        $this->assertSame(150, $config->targetDpi);
        $this->assertSame(2048, $config->maxImageDimension);
        $this->assertFalse($config->autoAdjustDpi);
    }

    #[Test]
    public function it_serializes_to_array_with_all_values(): void
    {
        $config = new ImageExtractionConfig(extractImages: true, targetDpi: 200);
        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_images']);
        $this->assertSame(200, $array['target_dpi']);
        $this->assertArrayHasKey('max_image_dimension', $array);
        $this->assertArrayHasKey('auto_adjust_dpi', $array);
    }

    #[Test]
    public function it_includes_dpi_settings_in_array(): void
    {
        $config = new ImageExtractionConfig(
            extractImages: true,
            minDpi: 100,
            maxDpi: 400,
        );
        $array = $config->toArray();

        $this->assertArrayHasKey('min_dpi', $array);
        $this->assertArrayHasKey('max_dpi', $array);
        $this->assertSame(100, $array['min_dpi']);
        $this->assertSame(400, $array['max_dpi']);
    }

    #[Test]
    public function it_creates_from_array_with_defaults(): void
    {
        $config = ImageExtractionConfig::fromArray([]);

        $this->assertTrue($config->extractImages);
        $this->assertSame(300, $config->targetDpi);
        $this->assertSame(4096, $config->maxImageDimension);
        $this->assertTrue($config->autoAdjustDpi);
    }

    #[Test]
    public function it_creates_from_array_with_all_fields(): void
    {
        $data = [
            'extract_images' => false,
            'target_dpi' => 150,
            'max_image_dimension' => 2048,
            'auto_adjust_dpi' => false,
            'min_dpi' => 50,
            'max_dpi' => 300,
        ];
        $config = ImageExtractionConfig::fromArray($data);

        $this->assertFalse($config->extractImages);
        $this->assertSame(150, $config->targetDpi);
        $this->assertSame(2048, $config->maxImageDimension);
        $this->assertFalse($config->autoAdjustDpi);
        $this->assertSame(50, $config->minDpi);
        $this->assertSame(300, $config->maxDpi);
    }

    #[Test]
    public function it_serializes_to_json(): void
    {
        $config = new ImageExtractionConfig(
            extractImages: true,
            targetDpi: 200,
            maxImageDimension: 3000,
        );
        $json = $config->toJson();

        $this->assertJson($json);
        $decoded = json_decode($json, true);

        $this->assertTrue($decoded['extract_images']);
        $this->assertSame(200, $decoded['target_dpi']);
        $this->assertSame(3000, $decoded['max_image_dimension']);
    }

    #[Test]
    public function it_creates_from_json(): void
    {
        $json = json_encode([
            'extract_images' => false,
            'target_dpi' => 250,
            'max_image_dimension' => 2500,
            'auto_adjust_dpi' => false,
        ]);
        $config = ImageExtractionConfig::fromJson($json);

        $this->assertFalse($config->extractImages);
        $this->assertSame(250, $config->targetDpi);
        $this->assertSame(2500, $config->maxImageDimension);
        $this->assertFalse($config->autoAdjustDpi);
    }

    #[Test]
    public function it_round_trips_through_json(): void
    {
        $original = new ImageExtractionConfig(
            extractImages: true,
            targetDpi: 350,
            maxImageDimension: 5000,
            autoAdjustDpi: false,
            minDpi: 80,
            maxDpi: 500,
        );

        $json = $original->toJson();
        $restored = ImageExtractionConfig::fromJson($json);

        $this->assertSame($original->extractImages, $restored->extractImages);
        $this->assertSame($original->targetDpi, $restored->targetDpi);
        $this->assertSame($original->maxImageDimension, $restored->maxImageDimension);
        $this->assertSame($original->autoAdjustDpi, $restored->autoAdjustDpi);
        $this->assertSame($original->minDpi, $restored->minDpi);
        $this->assertSame($original->maxDpi, $restored->maxDpi);
    }

    #[Test]
    public function it_throws_on_invalid_json(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('Invalid JSON');

        ImageExtractionConfig::fromJson('{ invalid }');
    }

    #[Test]
    public function it_enforces_readonly_on_extract_images_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ImageExtractionConfig(extractImages: true);
        $config->extractImages = false;
    }

    #[Test]
    public function it_enforces_readonly_on_target_dpi_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ImageExtractionConfig(targetDpi: 100);
        $config->targetDpi = 200;
    }

    #[Test]
    public function it_creates_from_file(): void
    {
        $tempFile = tempnam(sys_get_temp_dir(), 'img_');
        if ($tempFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tempFile, json_encode([
                'extract_images' => true,
                'target_dpi' => 250,
            ]));

            $config = ImageExtractionConfig::fromFile($tempFile);

            $this->assertTrue($config->extractImages);
            $this->assertSame(250, $config->targetDpi);
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

        ImageExtractionConfig::fromFile('/nonexistent/path/config.json');
    }

    #[Test]
    public function it_handles_type_coercion_for_extract_images(): void
    {
        $data = ['extract_images' => 1];
        $config = ImageExtractionConfig::fromArray($data);

        $this->assertIsBool($config->extractImages);
        $this->assertTrue($config->extractImages);
    }

    #[Test]
    public function it_handles_type_coercion_for_target_dpi(): void
    {
        $data = ['target_dpi' => '250'];
        $config = ImageExtractionConfig::fromArray($data);

        $this->assertIsInt($config->targetDpi);
        $this->assertSame(250, $config->targetDpi);
    }

    #[Test]
    public function it_uses_default_dpi_values(): void
    {
        $config = new ImageExtractionConfig();

        $this->assertSame(72, $config->minDpi);
        $this->assertSame(600, $config->maxDpi);
    }
}
