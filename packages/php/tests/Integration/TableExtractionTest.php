<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for table extraction functionality.
 *
 * Tests extraction of structured tables from various document types.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('tables')]
#[RequiresPhpExtension('kreuzberg')]
final class TableExtractionTest extends TestCase
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
    public function it_extracts_tables_from_pdf_with_tables(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        $this->assertIsArray(
            $result->tables,
            'Result should contain tables array',
        );
    }

    #[Test]
    public function it_provides_table_markdown_representation(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        if (!empty($result->tables)) {
            $table = $result->tables[0];

            $this->assertObjectHasProperty(
                'markdown',
                $table,
                'Table should have markdown representation',
            );
            $this->assertIsString(
                $table->markdown,
                'Markdown representation should be a string',
            );
            $this->assertNotEmpty(
                $table->markdown,
                'Markdown representation should not be empty',
            );
        }
    }

    #[Test]
    public function it_includes_table_page_numbers(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        if (!empty($result->tables)) {
            $table = $result->tables[0];

            $this->assertObjectHasProperty(
                'pageNumber',
                $table,
                'Table should have page number information',
            );
            $this->assertIsInt(
                $table->pageNumber,
                'Page number should be an integer',
            );
            $this->assertGreaterThan(
                0,
                $table->pageNumber,
                'Page number should be positive',
            );
        }
    }

    #[Test]
    public function it_disables_table_extraction_when_configured(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: false);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        $this->assertEmpty(
            $result->tables,
            'Tables should not be extracted when disabled in config',
        );
    }

    #[Test]
    public function it_extracts_tables_from_odt_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/simpleTable.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertIsArray(
            $result->tables,
            'ODT should have extractable tables',
        );
    }

    #[Test]
    public function it_extracts_tables_with_content(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/tableWithContents.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotEmpty(
            $result->content,
            'Document with tables should have extracted content',
        );
        $this->assertIsArray(
            $result->tables,
            'Should extract table structures',
        );
    }

    #[Test]
    public function it_extracts_multiple_tables_from_document(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);

        foreach ($pdfFiles as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            if (!empty($result->tables)) {
                $this->assertIsArray(
                    $result->tables,
                    'Multiple tables should be returned as array',
                );

                foreach ($result->tables as $table) {
                    $this->assertIsString(
                        $table->markdown,
                        'Each table should have markdown representation',
                    );
                }

                break;
            }
        }
    }

    #[Test]
    public function it_provides_table_data_structure(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        if (!empty($result->tables)) {
            $table = $result->tables[0];

            $this->assertObjectHasProperty(
                'data',
                $table,
                'Table should have data property',
            );
            $this->assertIsArray(
                $table->data,
                'Table data should be an array',
            );
        }
    }

    #[Test]
    public function it_handles_documents_without_tables(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertIsArray(
            $result->tables,
            'Documents without tables should have empty tables array',
        );
    }

    #[Test]
    public function it_extracts_tables_in_batch_processing(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (count($pdfFiles) < 2) {
            $this->markTestSkipped('Not enough PDF files with tables for batch test');
        }

        $files = array_slice($pdfFiles, 0, 2);

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(
            2,
            $results,
            'Batch processing should return results for all files',
        );

        foreach ($results as $result) {
            $this->assertIsArray(
                $result->tables,
                'Each result should have tables array',
            );
        }
    }

    #[Test]
    public function it_validates_table_structure_integrity(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        if (!empty($result->tables)) {
            $table = $result->tables[0];

            $this->assertObjectHasProperty('data', $table);
            $this->assertObjectHasProperty('markdown', $table);
            $this->assertObjectHasProperty('pageNumber', $table);

            if (!empty($table->data)) {
                $this->assertIsArray($table->data);

                foreach ($table->data as $row) {
                    $this->assertIsArray(
                        $row,
                        'Each table row should be an array',
                    );
                }
            }
        }
    }

    #[Test]
    public function it_preserves_table_formatting_in_markdown(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        if (!empty($result->tables)) {
            $markdown = $result->tables[0]->markdown;

            $this->assertStringContainsString(
                '|',
                $markdown,
                'Markdown table should contain pipe separators',
            );
        }
    }
}
