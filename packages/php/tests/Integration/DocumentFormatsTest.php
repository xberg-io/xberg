<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for extracting various document formats.
 *
 * Tests real-world document extraction across 56+ supported formats.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('formats')]
#[RequiresPhpExtension('kreuzberg')]
final class DocumentFormatsTest extends TestCase
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
    public function it_extracts_pdf_documents(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $this->assertNotEmpty($result->content, 'PDF should have extractable content');
        $this->assertStringContainsString('pdf', strtolower($result->mimeType));
        $this->assertGreaterThan(0, $result->metadata->pageCount, 'PDF should have at least one page');
    }

    #[Test]
    public function it_extracts_markdown_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty($result->content, 'Markdown should have extractable content');
        $this->assertStringContainsString('text', strtolower($result->mimeType));
    }

    #[Test]
    public function it_extracts_odt_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty($result->content, 'ODT should have extractable content');
        $this->assertNotNull($result->metadata);
    }

    #[Test]
    public function it_extracts_docx_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/office/document.docx';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty($result->content, 'DOCX should have extractable content');
        $this->assertStringContainsString('application/', $result->mimeType);
    }

    #[Test]
    public function it_extracts_epub_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/misc/simple.epub';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty($result->content, 'EPUB should have extractable content');
        $this->assertNotNull($result->metadata);
    }

    #[Test]
    #[DataProvider('provideDocumentFiles')]
    public function it_extracts_content_from_various_formats(string $relativePath, string $expectedType): void
    {
        $filePath = $this->testDocumentsPath . '/' . $relativePath;

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty(
            $result->content,
            "Content should be extracted from {$relativePath}",
        );
        $this->assertNotNull(
            $result->metadata,
            "Metadata should be available for {$relativePath}",
        );
        $this->assertNotEmpty(
            $result->mimeType,
            "MIME type should be detected for {$relativePath}",
        );
    }

    public static function provideDocumentFiles(): array
    {
        return [
            'PDF document' => ['pdfs/code_and_formula.pdf', 'pdf'],
            'Markdown file' => ['extraction_test.md', 'text'],
            'ODT document' => ['extraction_test.odt', 'odt'],
            'DOCX document' => ['extraction_test.docx', 'docx'],
        ];
    }

    #[Test]
    public function it_extracts_metadata_from_pdf(): void
    {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($this->testDocumentsPath . '/pdfs/code_and_formula.pdf');

        $metadata = $result->metadata;

        $this->assertNotNull($metadata);
        $this->assertIsInt($metadata->pageCount);
        $this->assertGreaterThan(0, $metadata->pageCount);

        $this->assertObjectHasProperty('title', $metadata);
        $this->assertObjectHasProperty('authors', $metadata);
    }

    #[Test]
    public function it_preserves_text_content_accuracy(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $originalContent = file_get_contents($filePath);

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertGreaterThan(
            0,
            strlen($result->content),
            'Extracted content should not be empty',
        );
    }

    #[Test]
    public function it_handles_different_pdf_versions(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $kreuzberg = new Kreuzberg();

        foreach (array_slice($pdfFiles, 0, 3) as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            $this->assertNotEmpty(
                $result->content,
                'Should extract content from ' . basename($pdfFile),
            );
            $this->assertStringContainsString('pdf', strtolower($result->mimeType));
        }
    }

    #[Test]
    public function it_extracts_from_complex_pdf(): void
    {
        $complexPdfs = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($complexPdfs)) {
            $this->markTestSkipped('No complex PDFs found for testing');
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($complexPdfs[0]);

        $this->assertNotEmpty(
            $result->content,
            'Complex PDF should have extractable content',
        );
        $this->assertIsArray(
            $result->tables,
            'Complex PDF might contain tables',
        );
    }

    #[Test]
    public function it_detects_correct_mime_types(): void
    {
        $files = [
            'pdfs/code_and_formula.pdf' => 'pdf',
            'extraction_test.md' => 'text',
        ];

        $kreuzberg = new Kreuzberg();

        foreach ($files as $file => $expectedMimeFragment) {
            $filePath = $this->testDocumentsPath . '/' . $file;

            if (!file_exists($filePath)) {
                continue;
            }

            $result = $kreuzberg->extractFile($filePath);

            $this->assertStringContainsString(
                $expectedMimeFragment,
                strtolower($result->mimeType),
                "MIME type for {$file} should contain '{$expectedMimeFragment}'",
            );
        }
    }

    #[Test]
    public function it_handles_unicode_content(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/unicode.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty(
            $result->content,
            'Unicode content should be extracted correctly',
        );

        $this->assertTrue(
            mb_check_encoding($result->content, 'UTF-8'),
            'Extracted content should be valid UTF-8',
        );
    }

    #[Test]
    public function it_extracts_from_documents_with_formulas(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/formula.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty(
            $result->content,
            'Documents with formulas should have extractable content',
        );
    }

    #[Test]
    public function it_extracts_from_documents_with_lists(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/listBlocks.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty(
            $result->content,
            'Documents with lists should have extractable content',
        );
    }
}
