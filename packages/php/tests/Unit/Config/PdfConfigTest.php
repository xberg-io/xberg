<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit\Config;

use Kreuzberg\Config\PdfConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for PdfConfig readonly class.
 *
 * Tests construction, serialization, factory methods, readonly enforcement,
 * and handling of boolean and nullable properties.
 */
#[CoversClass(PdfConfig::class)]
#[Group('unit')]
#[Group('config')]
final class PdfConfigTest extends TestCase
{
    #[Test]
    public function it_creates_with_default_values(): void
    {
        $config = new PdfConfig();

        $this->assertFalse($config->extractImages);
        $this->assertTrue($config->extractMetadata);
        $this->assertNull($config->passwords);
        $this->assertNull($config->hierarchy);
    }

    #[Test]
    public function it_creates_with_custom_values(): void
    {
        $config = new PdfConfig(
            extractImages: true,
            extractMetadata: false,
            passwords: ['password123'],
        );

        $this->assertTrue($config->extractImages);
        $this->assertFalse($config->extractMetadata);
        $this->assertSame(['password123'], $config->passwords);
    }

    #[Test]
    public function it_serializes_to_array_with_only_non_default_values(): void
    {
        $config = new PdfConfig(extractImages: true);
        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_images']);
        $this->assertTrue($array['extract_metadata']);
    }

    #[Test]
    public function it_creates_from_array_with_defaults(): void
    {
        $config = PdfConfig::fromArray([]);

        $this->assertFalse($config->extractImages);
        $this->assertTrue($config->extractMetadata);
        $this->assertNull($config->passwords);
    }

    #[Test]
    public function it_creates_from_array_with_all_fields(): void
    {
        $data = [
            'extract_images' => true,
            'extract_metadata' => false,
            'passwords' => ['pass1', 'pass2'],
        ];
        $config = PdfConfig::fromArray($data);

        $this->assertTrue($config->extractImages);
        $this->assertFalse($config->extractMetadata);
        $this->assertSame(['pass1', 'pass2'], $config->passwords);
    }

    #[Test]
    public function it_serializes_to_json(): void
    {
        $config = new PdfConfig(
            extractImages: true,
            extractMetadata: true,
            passwords: ['mypass'],
        );
        $json = $config->toJson();

        $this->assertJson($json);
        $decoded = json_decode($json, true);

        $this->assertTrue($decoded['extract_images']);
        $this->assertTrue($decoded['extract_metadata']);
        $this->assertSame(['mypass'], $decoded['passwords']);
    }

    #[Test]
    public function it_creates_from_json(): void
    {
        $json = json_encode([
            'extract_images' => false,
            'extract_metadata' => true,
            'passwords' => ['test'],
        ]);
        $config = PdfConfig::fromJson($json);

        $this->assertFalse($config->extractImages);
        $this->assertTrue($config->extractMetadata);
        $this->assertSame(['test'], $config->passwords);
    }

    #[Test]
    public function it_round_trips_through_json(): void
    {
        $original = new PdfConfig(
            extractImages: true,
            extractMetadata: false,
            passwords: ['abc', 'def'],
        );

        $json = $original->toJson();
        $restored = PdfConfig::fromJson($json);

        $this->assertSame($original->extractImages, $restored->extractImages);
        $this->assertSame($original->extractMetadata, $restored->extractMetadata);
        $this->assertSame($original->passwords, $restored->passwords);
    }

    #[Test]
    public function it_throws_on_invalid_json(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('Invalid JSON');

        PdfConfig::fromJson('{ invalid }');
    }

    #[Test]
    public function it_enforces_readonly_on_extract_images_property(): void
    {
        $this->expectException(\Error::class);

        $config = new PdfConfig(extractImages: true);
        $config->extractImages = false;
    }

    #[Test]
    public function it_enforces_readonly_on_extract_metadata_property(): void
    {
        $this->expectException(\Error::class);

        $config = new PdfConfig(extractMetadata: true);
        $config->extractMetadata = false;
    }

    #[Test]
    public function it_creates_from_file(): void
    {
        $tempFile = tempnam(sys_get_temp_dir(), 'pdf_');
        if ($tempFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tempFile, json_encode([
                'extract_images' => true,
                'passwords' => ['test123'],
            ]));

            $config = PdfConfig::fromFile($tempFile);

            $this->assertTrue($config->extractImages);
            $this->assertSame(['test123'], $config->passwords);
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

        PdfConfig::fromFile('/nonexistent/path/config.json');
    }

    #[Test]
    public function it_handles_type_coercion_for_extract_images(): void
    {
        $data = ['extract_images' => 1];
        $config = PdfConfig::fromArray($data);

        $this->assertIsBool($config->extractImages);
        $this->assertTrue($config->extractImages);
    }

    #[Test]
    public function it_handles_null_passwords(): void
    {
        $config = new PdfConfig(passwords: null);

        $this->assertNull($config->passwords);
    }

    #[Test]
    public function it_handles_empty_passwords_array(): void
    {
        $config = new PdfConfig(passwords: []);

        $this->assertSame([], $config->passwords);
    }
}
