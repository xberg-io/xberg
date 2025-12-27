<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for document extraction functionality.
 *
 * Tests the core extraction API including file extraction, byte extraction,
 * and batch operations. These tests require the Kreuzberg extension to be loaded.
 */
#[CoversClass(Kreuzberg::class)]
final class ExtractionTest extends TestCase
{
    #[Test]
    public function it_returns_library_version(): void
    {
        $version = Kreuzberg::version();

        $this->assertIsString($version);
        $this->assertMatchesRegularExpression('/^\d+\.\d+\.\d+/', $version);
    }

    #[Test]
    public function it_creates_kreuzberg_instance_without_config(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_creates_kreuzberg_instance_with_config(): void
    {
        $config = new ExtractionConfig(
            extractImages: true,
            extractTables: false,
        );

        $kreuzberg = new Kreuzberg($config);

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_has_correct_version_constant(): void
    {
        $this->assertSame('4.0.0-rc.20', Kreuzberg::VERSION);
    }

    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function it_checks_if_extension_is_loaded(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->assertTrue(extension_loaded('kreuzberg'));
        $this->assertIsString(phpversion('kreuzberg'));
    }

    #[Test]
    public function it_validates_extraction_config_is_optional(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_is_readonly_class(): void
    {
        $kreuzberg = new Kreuzberg();

        $reflection = new \ReflectionClass($kreuzberg);
        $this->assertTrue($reflection->isReadOnly());
        $this->assertTrue($reflection->isFinal());
    }

    #[Test]
    public function it_accepts_null_default_config(): void
    {
        $kreuzberg = new Kreuzberg(null);

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_has_extract_file_method(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertTrue(method_exists($kreuzberg, 'extractFile'));
    }

    #[Test]
    public function it_has_extract_bytes_method(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertTrue(method_exists($kreuzberg, 'extractBytes'));
    }

    #[Test]
    public function it_has_batch_extract_files_method(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertTrue(method_exists($kreuzberg, 'batchExtractFiles'));
    }

    #[Test]
    public function it_has_batch_extract_bytes_method(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertTrue(method_exists($kreuzberg, 'batchExtractBytes'));
    }

    #[Test]
    public function it_has_static_version_method(): void
    {
        $this->assertTrue(method_exists(Kreuzberg::class, 'version'));
    }

    #[Test]
    public function it_validates_proper_method_signatures(): void
    {
        $reflection = new \ReflectionClass(Kreuzberg::class);

        $extractFile = $reflection->getMethod('extractFile');
        $this->assertSame(3, $extractFile->getNumberOfParameters());
        $this->assertSame(1, $extractFile->getNumberOfRequiredParameters());

        $extractBytes = $reflection->getMethod('extractBytes');
        $this->assertSame(3, $extractBytes->getNumberOfParameters());
        $this->assertSame(2, $extractBytes->getNumberOfRequiredParameters());

        $batchExtractFiles = $reflection->getMethod('batchExtractFiles');
        $this->assertSame(2, $batchExtractFiles->getNumberOfParameters());
        $this->assertSame(1, $batchExtractFiles->getNumberOfRequiredParameters());

        $batchExtractBytes = $reflection->getMethod('batchExtractBytes');
        $this->assertSame(3, $batchExtractBytes->getNumberOfParameters());
        $this->assertSame(2, $batchExtractBytes->getNumberOfRequiredParameters());
    }

    #[Test]
    public function it_validates_constructor_signature(): void
    {
        $reflection = new \ReflectionClass(Kreuzberg::class);
        $constructor = $reflection->getConstructor();

        $this->assertNotNull($constructor);
        $this->assertSame(1, $constructor->getNumberOfParameters());
        $this->assertSame(0, $constructor->getNumberOfRequiredParameters());
    }
}
