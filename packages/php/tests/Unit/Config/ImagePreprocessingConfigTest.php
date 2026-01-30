<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit\Config;

use Kreuzberg\Config\ImagePreprocessingConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for ImagePreprocessingConfig readonly class.
 *
 * Tests construction, serialization, factory methods, readonly enforcement,
 * and handling of mixed property types (int, bool, string).
 */
#[CoversClass(ImagePreprocessingConfig::class)]
#[Group('unit')]
#[Group('config')]
final class ImagePreprocessingConfigTest extends TestCase
{
    #[Test]
    public function it_creates_with_default_values(): void
    {
        $config = new ImagePreprocessingConfig();

        $this->assertSame(300, $config->targetDpi);
        $this->assertTrue($config->autoRotate);
        $this->assertTrue($config->deskew);
        $this->assertFalse($config->denoise);
        $this->assertFalse($config->contrastEnhance);
        $this->assertSame('otsu', $config->binarizationMethod);
        $this->assertFalse($config->invertColors);
    }

    #[Test]
    public function it_creates_with_custom_values(): void
    {
        $config = new ImagePreprocessingConfig(
            targetDpi: 600,
            autoRotate: false,
            deskew: false,
            denoise: true,
            contrastEnhance: true,
            binarizationMethod: 'sauvola',
            invertColors: true,
        );

        $this->assertSame(600, $config->targetDpi);
        $this->assertFalse($config->autoRotate);
        $this->assertFalse($config->deskew);
        $this->assertTrue($config->denoise);
        $this->assertTrue($config->contrastEnhance);
        $this->assertSame('sauvola', $config->binarizationMethod);
        $this->assertTrue($config->invertColors);
    }

    #[Test]
    public function it_serializes_to_array(): void
    {
        $config = new ImagePreprocessingConfig(
            targetDpi: 200,
            denoise: true,
            binarizationMethod: 'adaptive',
        );
        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertArrayHasKey('target_dpi', $array);
        $this->assertArrayHasKey('auto_rotate', $array);
        $this->assertArrayHasKey('deskew', $array);
        $this->assertArrayHasKey('denoise', $array);
        $this->assertArrayHasKey('contrast_enhance', $array);
        $this->assertArrayHasKey('binarization_method', $array);
        $this->assertArrayHasKey('invert_colors', $array);
        $this->assertSame(200, $array['target_dpi']);
        $this->assertTrue($array['denoise']);
        $this->assertSame('adaptive', $array['binarization_method']);
    }

    #[Test]
    public function it_creates_from_array_with_defaults(): void
    {
        $config = ImagePreprocessingConfig::fromArray([]);

        $this->assertSame(300, $config->targetDpi);
        $this->assertTrue($config->autoRotate);
        $this->assertTrue($config->deskew);
        $this->assertFalse($config->denoise);
        $this->assertFalse($config->contrastEnhance);
        $this->assertSame('otsu', $config->binarizationMethod);
        $this->assertFalse($config->invertColors);
    }

    #[Test]
    public function it_creates_from_array_with_all_fields(): void
    {
        $data = [
            'target_dpi' => 600,
            'auto_rotate' => false,
            'deskew' => false,
            'denoise' => true,
            'contrast_enhance' => true,
            'binarization_method' => 'sauvola',
            'invert_colors' => true,
        ];
        $config = ImagePreprocessingConfig::fromArray($data);

        $this->assertSame(600, $config->targetDpi);
        $this->assertFalse($config->autoRotate);
        $this->assertFalse($config->deskew);
        $this->assertTrue($config->denoise);
        $this->assertTrue($config->contrastEnhance);
        $this->assertSame('sauvola', $config->binarizationMethod);
        $this->assertTrue($config->invertColors);
    }

    #[Test]
    public function it_serializes_to_json(): void
    {
        $config = new ImagePreprocessingConfig(
            targetDpi: 300,
            denoise: true,
            contrastEnhance: true,
        );
        $json = $config->toJson();

        $this->assertJson($json);
        $decoded = json_decode($json, true);

        $this->assertSame(300, $decoded['target_dpi']);
        $this->assertTrue($decoded['denoise']);
        $this->assertTrue($decoded['contrast_enhance']);
    }

    #[Test]
    public function it_creates_from_json(): void
    {
        $json = json_encode([
            'target_dpi' => 150,
            'deskew' => false,
            'denoise' => true,
            'binarization_method' => 'adaptive',
        ]);
        $config = ImagePreprocessingConfig::fromJson($json);

        $this->assertSame(150, $config->targetDpi);
        $this->assertFalse($config->deskew);
        $this->assertTrue($config->denoise);
        $this->assertSame('adaptive', $config->binarizationMethod);
    }

    #[Test]
    public function it_round_trips_through_json(): void
    {
        $original = new ImagePreprocessingConfig(
            targetDpi: 600,
            autoRotate: false,
            deskew: false,
            denoise: true,
            contrastEnhance: true,
            binarizationMethod: 'sauvola',
            invertColors: true,
        );

        $json = $original->toJson();
        $restored = ImagePreprocessingConfig::fromJson($json);

        $this->assertSame($original->targetDpi, $restored->targetDpi);
        $this->assertSame($original->autoRotate, $restored->autoRotate);
        $this->assertSame($original->deskew, $restored->deskew);
        $this->assertSame($original->denoise, $restored->denoise);
        $this->assertSame($original->contrastEnhance, $restored->contrastEnhance);
        $this->assertSame($original->binarizationMethod, $restored->binarizationMethod);
        $this->assertSame($original->invertColors, $restored->invertColors);
    }

    #[Test]
    public function it_throws_on_invalid_json(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('Invalid JSON');

        ImagePreprocessingConfig::fromJson('{ invalid }');
    }

    #[Test]
    public function it_enforces_readonly_on_target_dpi_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ImagePreprocessingConfig(targetDpi: 300);
        $config->targetDpi = 200;
    }

    #[Test]
    public function it_enforces_readonly_on_auto_rotate_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ImagePreprocessingConfig(autoRotate: true);
        $config->autoRotate = false;
    }

    #[Test]
    public function it_enforces_readonly_on_binarization_method_property(): void
    {
        $this->expectException(\Error::class);

        $config = new ImagePreprocessingConfig(binarizationMethod: 'otsu');
        $config->binarizationMethod = 'sauvola';
    }

    #[Test]
    public function it_creates_from_file(): void
    {
        $tempFile = tempnam(sys_get_temp_dir(), 'imgprep_');
        if ($tempFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tempFile, json_encode([
                'target_dpi' => 300,
                'auto_rotate' => false,
                'binarization_method' => 'adaptive',
            ]));

            $config = ImagePreprocessingConfig::fromFile($tempFile);

            $this->assertSame(300, $config->targetDpi);
            $this->assertFalse($config->autoRotate);
            $this->assertSame('adaptive', $config->binarizationMethod);
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

        ImagePreprocessingConfig::fromFile('/nonexistent/path/config.json');
    }

    #[Test]
    public function it_handles_type_coercion_for_target_dpi(): void
    {
        $data = ['target_dpi' => '300'];
        $config = ImagePreprocessingConfig::fromArray($data);

        $this->assertIsInt($config->targetDpi);
        $this->assertSame(300, $config->targetDpi);
    }

    #[Test]
    public function it_handles_type_coercion_for_bool_values(): void
    {
        $data = [
            'auto_rotate' => 0,
            'denoise' => '1',
            'deskew' => 'true',
        ];
        $config = ImagePreprocessingConfig::fromArray($data);

        $this->assertIsBool($config->autoRotate);
        $this->assertFalse($config->autoRotate);
        $this->assertIsBool($config->denoise);
        $this->assertTrue($config->denoise);
        $this->assertIsBool($config->deskew);
        $this->assertTrue($config->deskew);
    }
}
