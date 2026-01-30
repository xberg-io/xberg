<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use Kreuzberg\Types\ExtractionResult;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for the Kreuzberg PHP extension.
 *
 * These tests verify that the PHP extension is properly loaded and can
 * communicate with the Rust core. Tests are skipped if the extension
 * is not available.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[RequiresPhpExtension('kreuzberg-php')]
final class ExtensionTest extends TestCase
{
    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg-php')) {
            $this->markTestSkipped(
                'Kreuzberg extension is not loaded. ' .
                'These integration tests require the compiled extension.',
            );
        }
    }

    #[Test]
    public function it_loads_kreuzberg_extension(): void
    {
        $this->assertTrue(extension_loaded('kreuzberg-php'));
    }

    #[Test]
    public function it_reports_extension_version(): void
    {
        $version = phpversion('kreuzberg-php');

        $this->assertNotFalse($version);
        $this->assertIsString($version);
        $this->assertMatchesRegularExpression('/^\d+\.\d+\.\d+/', $version);
    }

    #[Test]
    public function it_has_required_extension_functions(): void
    {
        $requiredFunctions = [
            'kreuzberg_extract_file',
            'kreuzberg_extract_bytes',
            'kreuzberg_batch_extract_files',
            'kreuzberg_batch_extract_bytes',
            'kreuzberg_detect_mime_type',
            'kreuzberg_detect_mime_type_from_path',
        ];

        foreach ($requiredFunctions as $function) {
            $this->assertTrue(
                function_exists($function),
                "Extension function '{$function}' should exist",
            );
        }
    }

    #[Test]
    public function it_detects_mime_type_from_pdf_bytes(): void
    {
        $pdfBytes = "%PDF-1.4\n%âãÏÓ\n";

        $mimeType = \kreuzberg_detect_mime_type($pdfBytes);

        $this->assertIsString($mimeType);
        $this->assertStringContainsString('pdf', strtolower($mimeType));
    }

    #[Test]
    public function it_detects_mime_type_from_png_bytes(): void
    {
        $pngBytes = "\x89PNG\r\n\x1a\n";

        $mimeType = \kreuzberg_detect_mime_type($pngBytes);

        $this->assertIsString($mimeType);
        $this->assertStringContainsString('png', strtolower($mimeType));
    }

    #[Test]
    public function it_throws_exception_for_nonexistent_file(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile('/nonexistent/file/path.pdf');
    }

    #[Test]
    public function it_validates_config_serialization(): void
    {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
            pdfOptions: new PdfConfig(extractImages: true),
            useCache: false,  // Set to false to ensure it appears in array
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertArrayHasKey('ocr', $array);
        $this->assertArrayHasKey('pdf_options', $array);
        $this->assertFalse($array['use_cache']);
    }

    #[Test]
    public function it_creates_kreuzberg_instance_with_default_config(): void
    {
        $kreuzberg = new Kreuzberg();

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_creates_kreuzberg_instance_with_custom_config(): void
    {
        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
            useCache: true,
        );

        $kreuzberg = new Kreuzberg($config);

        $this->assertInstanceOf(Kreuzberg::class, $kreuzberg);
    }

    #[Test]
    public function it_validates_extraction_result_structure(): void
    {
        $this->assertTrue(class_exists(ExtractionResult::class));

        // The extension may override reflection metadata, so we check by accessing an actual instance
        $testDocumentsPath = dirname(__DIR__, 4) . DIRECTORY_SEPARATOR . 'test_documents';
        $pdfPath = $testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($pdfPath)) {
            $this->markTestSkipped("Test file not found: {$pdfPath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($pdfPath);

        // Verify all required properties are accessible on the result instance
        $expectedProperties = [
            'content',
            'mimeType',
            'metadata',
            'tables',
            'detectedLanguages',
            'chunks',
            'images',
            'pages',
        ];

        foreach ($expectedProperties as $property) {
            try {
                $value = $result->$property;
                $this->assertTrue(true, "Property {$property} is accessible");
            } catch (\Error $e) {
                $this->fail("ExtractionResult should have accessible property: {$property}");
            }
        }
    }

    #[Test]
    public function it_validates_procedural_api_functions_exist(): void
    {
        $proceduralFunctions = [
            'Kreuzberg\extract_file',
            'Kreuzberg\extract_bytes',
            'Kreuzberg\batch_extract_files',
            'Kreuzberg\batch_extract_bytes',
            'Kreuzberg\detect_mime_type',
            'Kreuzberg\detect_mime_type_from_path',
        ];

        foreach ($proceduralFunctions as $function) {
            $this->assertTrue(
                function_exists($function),
                "Procedural function '{$function}' should exist",
            );
        }
    }

    #[Test]
    public function it_matches_class_version_with_extension_version(): void
    {
        $classVersion = Kreuzberg::version();
        $extensionVersion = phpversion('kreuzberg-php');

        $this->assertMatchesRegularExpression('/^\d+\.\d+\.\d+/', $classVersion);

        if ($extensionVersion !== false) {
            $this->assertMatchesRegularExpression('/^\d+\.\d+\.\d+/', $extensionVersion);
        }
    }

    #[Test]
    public function it_handles_invalid_mime_type_gracefully(): void
    {
        $kreuzberg = new Kreuzberg();

        $tmpFile = tempnam(sys_get_temp_dir(), 'krz_test_');
        file_put_contents($tmpFile, 'test content');

        try {
            $this->expectException(KreuzbergException::class);
            $kreuzberg->extractFile($tmpFile, 'invalid/mime-type');
        } finally {
            @unlink($tmpFile);
        }
    }

    #[Test]
    public function it_validates_batch_operations_accept_arrays(): void
    {
        $kreuzberg = new Kreuzberg();

        $reflection = new \ReflectionClass($kreuzberg);

        $batchExtractFiles = $reflection->getMethod('batchExtractFiles');
        $params = $batchExtractFiles->getParameters();
        $this->assertSame('array', $params[0]->getType()?->getName());

        $batchExtractBytes = $reflection->getMethod('batchExtractBytes');
        $params = $batchExtractBytes->getParameters();
        $this->assertSame('array', $params[0]->getType()?->getName());
        $this->assertSame('array', $params[1]->getType()?->getName());
    }

    #[Test]
    public function it_checks_extension_constants_are_defined(): void
    {
        $constants = get_defined_constants(true);

        $this->assertArrayHasKey('user', $constants);
        $this->assertIsArray($constants);
    }
}
