<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Behavior-driven unit tests for document extraction functionality.
 *
 * Tests focus on observable user-facing behavior, not implementation details.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('unit')]
#[RequiresPhpExtension('kreuzberg')]
final class DocumentExtractionTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . '/test_documents';
    }

    #[Test]
    public function it_extracts_text_from_simple_pdf(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertNotEmpty($result->content, 'Extracted content should not be empty');
        $this->assertIsString($result->content);
        $this->assertSame('application/pdf', $result->mimeType);
    }

    #[Test]
    public function it_extracts_text_from_office_document(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/office/document.docx');

        $this->assertNotEmpty($result->content, 'Extracted content from DOCX should not be empty');
        $this->assertStringContainsString('application/', $result->mimeType);
    }

    #[Test]
    public function it_extracts_content_from_bytes(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        $bytes = file_get_contents($filePath);

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractBytes($bytes, 'application/pdf');

        $this->assertNotEmpty($result->content, 'Extraction from bytes should produce content');
        $this->assertSame('application/pdf', $result->mimeType);
    }

    #[Test]
    public function it_throws_exception_for_nonexistent_file(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile('/nonexistent/path/to/file.pdf');
    }

    #[Test]
    public function it_throws_exception_for_empty_file_path(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile('');
    }

    #[Test]
    public function it_applies_default_config_when_none_provided(): void
    {
        $defaultConfig = new ExtractionConfig(extractTables: false);
        $kreuzberg = new Kreuzberg($defaultConfig);

        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertEmpty($result->tables, 'Tables should not be extracted when config disables them');
    }

    #[Test]
    public function it_overrides_default_config_with_method_config(): void
    {
        $defaultConfig = new ExtractionConfig(extractTables: false);
        $kreuzberg = new Kreuzberg($defaultConfig);

        $overrideConfig = new ExtractionConfig(extractTables: true);
        $result = $kreuzberg->extractFile(
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            config: $overrideConfig,
        );

        $this->assertNotNull($result->content);
    }

    #[Test]
    public function it_extracts_metadata_from_document(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertNotNull($result->metadata, 'Metadata should be present');
        $this->assertIsInt($result->metadata->pageCount);
        $this->assertGreaterThan(0, $result->metadata->pageCount, 'Page count should be positive');
    }

    #[Test]
    public function it_auto_detects_mime_type_when_not_provided(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertStringContainsString(
            'pdf',
            strtolower($result->mimeType),
            'MIME type should be auto-detected as PDF',
        );
    }

    #[Test]
    public function it_accepts_explicit_mime_type_hint(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile(
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            mimeType: 'application/pdf',
        );

        $this->assertSame('application/pdf', $result->mimeType);
    }

    #[Test]
    public function it_extracts_different_document_formats(): void
    {
        $kreuzberg = new Kreuzberg();

        $testFiles = [
            'pdfs/code_and_formula.pdf' => 'pdf',
            'extraction_test.md' => 'text',
        ];

        foreach ($testFiles as $file => $expectedType) {
            $filePath = $this->testDocumentsPath . '/' . $file;

            if (!file_exists($filePath)) {
                $this->markTestSkipped("Test file not found: {$filePath}");
            }

            $result = $kreuzberg->extractFile($filePath);
            $this->assertNotEmpty($result->content, "Content should be extracted from {$file}");
            $this->assertStringContainsString(
                $expectedType,
                strtolower($result->mimeType),
                "MIME type should contain '{$expectedType}' for {$file}",
            );
        }
    }

    #[Test]
    public function it_provides_library_version(): void
    {
        $version = Kreuzberg::version();

        $this->assertIsString($version);
        $this->assertMatchesRegularExpression(
            '/^\d+\.\d+\.\d+/',
            $version,
            'Version should follow semantic versioning format',
        );
    }

    #[Test]
    public function it_handles_extraction_without_default_config(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertNotEmpty($result->content);
        $this->assertIsArray($result->tables);
    }

    #[Test]
    public function it_handles_null_default_config_explicitly(): void
    {
        $kreuzberg = new Kreuzberg(null);
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertNotEmpty($result->content, 'Extraction should work with explicit null config');
    }

    #[Test]
    public function it_returns_empty_tables_array_when_no_tables_found(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/extraction_test.md');

        $this->assertIsArray($result->tables);
    }

    #[Test]
    public function it_preserves_extraction_result_immutability(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $reflection = new \ReflectionClass($result);
        $this->assertTrue(
            $reflection->isReadOnly(),
            'ExtractionResult should be immutable (readonly)',
        );
    }
}
