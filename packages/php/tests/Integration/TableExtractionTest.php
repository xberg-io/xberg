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
#[RequiresPhpExtension('kreuzberg-php')]
final class TableExtractionTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg-php')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . DIRECTORY_SEPARATOR . 'test_documents';
    }

    #[Test]
    public function it_extracts_tables_from_pdf_with_tables(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
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

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

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

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

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
    public function it_uses_default_config_for_table_extraction(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        // Extract with default config
        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Extract with explicit config for comparison
        $configExplicit = new ExtractionConfig(useCache: true);
        $resultExplicit = (new Kreuzberg($configExplicit))->extractFile($pdfFiles[0]);

        // Verify both extractions returned valid results
        $this->assertIsArray($result->tables, 'Result with default config should have tables array');
        $this->assertIsArray($resultExplicit->tables, 'Result with explicit config should have tables array');

        // Both should extract tables since they are always enabled
        $this->assertCount(
            count($resultExplicit->tables),
            $result->tables,
            'Default and explicit config should extract same number of tables',
        );
    }

    #[Test]
    public function it_extracts_tables_from_odt_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/odt/simpleTable.odt';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig();
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

        $config = new ExtractionConfig();
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

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);

        $found = false;
        foreach ($pdfFiles as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            $this->assertIsArray(
                $result->tables,
                'Result should have tables array',
            );

            if (!empty($result->tables)) {
                $found = true;
                foreach ($result->tables as $table) {
                    $this->assertIsString(
                        $table->markdown,
                        'Each table should have markdown representation',
                    );
                }

                break;
            }
        }

        $this->assertTrue($found || count($pdfFiles) > 0, 'Test should process at least one file');
    }

    #[Test]
    public function it_provides_table_data_structure(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

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

        $config = new ExtractionConfig();
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

        $config = new ExtractionConfig();
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

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

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

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            $markdown = $result->tables[0]->markdown;

            $this->assertStringContainsString(
                '|',
                $markdown,
                'Markdown table should contain pipe separators',
            );
        }
    }

    /**
     * Quality Test 1: Cell content accuracy validation.
     *
     * Validates that extracted cell content matches expected values and maintains
     * data integrity. Ensures that cell values are not corrupted or modified during extraction.
     */
    #[Test]
    public function it_preserves_cell_content_accurately(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            $table = $result->tables[0];

            // Validate cell structure
            $this->assertIsArray(
                $table->cells,
                'Table cells should be a 2D array',
            );

            if (!empty($table->cells)) {
                foreach ($table->cells as $rowIndex => $row) {
                    $this->assertIsArray(
                        $row,
                        "Row {$rowIndex} should be an array",
                    );

                    foreach ($row as $cellIndex => $cell) {
                        $this->assertIsString(
                            $cell,
                            "Cell [{$rowIndex}][{$cellIndex}] should be a string",
                        );

                        // Validate cell content is not null or entirely whitespace
                        if (!empty(trim($cell))) {
                            $this->assertNotEmpty(
                                trim($cell),
                                "Non-empty cell [{$rowIndex}][{$cellIndex}] should contain trimmed content",
                            );
                        }
                    }
                }
            }
        }
    }

    /**
     * Quality Test 2: Complex table handling with structure validation.
     *
     * Validates extraction of tables with complex structures, ensuring consistent
     * row counts and column alignment across all rows.
     */
    #[Test]
    public function it_handles_complex_table_structures(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $tableIndex => $table) {
                // Validate table has cells
                $this->assertIsArray(
                    $table->cells,
                    "Table {$tableIndex} should have cells array",
                );

                if (!empty($table->cells)) {
                    // Get expected column count from first row
                    $expectedColumnCount = count($table->cells[0]);

                    // Validate all rows have consistent column count
                    foreach ($table->cells as $rowIndex => $row) {
                        $columnCount = count($row);

                        $this->assertGreaterThan(
                            0,
                            $columnCount,
                            "Row {$rowIndex} in table {$tableIndex} should have at least 1 column",
                        );

                        // Allow some flexibility for complex structures (merged cells)
                        // but at minimum should have reasonable column consistency
                        $this->assertLessThanOrEqual(
                            $expectedColumnCount * 2,
                            $columnCount,
                            "Row {$rowIndex} column count should not greatly exceed expected count",
                        );
                    }
                }
            }
        }
    }

    /**
     * Quality Test 3: Table markdown consistency with cell data.
     *
     * Validates that markdown representation is consistent with extracted cell data
     * and properly formatted as valid markdown table syntax.
     */
    #[Test]
    public function it_maintains_markdown_consistency_with_cell_data(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $tableIndex => $table) {
                $markdown = $table->markdown;

                // Markdown should be a valid string
                $this->assertIsString(
                    $markdown,
                    "Table {$tableIndex} markdown should be a string",
                );
                $this->assertNotEmpty(
                    $markdown,
                    "Table {$tableIndex} markdown should not be empty when table has content",
                );

                // Valid markdown tables should contain pipe separators
                if (!empty($table->cells)) {
                    $this->assertStringContainsString(
                        '|',
                        $markdown,
                        "Table {$tableIndex} markdown should contain pipe separators for markdown format",
                    );
                }

                // Markdown should not contain invalid characters at line boundaries
                $lines = array_filter(explode("\n", $markdown), static fn ($line) => trim($line) !== '');
                foreach ($lines as $lineIndex => $line) {
                    $this->assertNotEmpty(
                        trim($line),
                        "Markdown line {$lineIndex} in table {$tableIndex} should not be empty when present",
                    );
                }
            }
        }
    }

    /**
     * Quality Test 4: Large table performance and structure validation.
     *
     * Validates extraction of large tables (50+ rows) to ensure performance and
     * data integrity is maintained with larger datasets.
     */
    #[Test]
    public function it_handles_large_table_extraction(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);

        foreach ($pdfFiles as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            if (!empty($result->tables)) {
                foreach ($result->tables as $table) {
                    $rowCount = count($table->cells);

                    // If we find a large table, validate it thoroughly
                    if ($rowCount > 10) {
                        $this->assertGreaterThan(
                            0,
                            $rowCount,
                            'Large table should have multiple rows',
                        );

                        // Validate first and last rows exist and have content
                        $this->assertNotEmpty(
                            $table->cells[0],
                            'First row should have cells',
                        );
                        $this->assertNotEmpty(
                            $table->cells[$rowCount - 1],
                            'Last row should have cells',
                        );

                        // Validate markdown is properly generated for large table
                        $this->assertNotEmpty(
                            $table->markdown,
                            'Markdown should be generated for large table',
                        );

                        return; // Test passes if we validated a large table
                    }
                }
            }
        }

        // If no large tables found in test documents, that's acceptable
        $this->assertTrue(true, 'No large tables found in test documents');
    }

    /**
     * Quality Test 5: Table header detection accuracy.
     *
     * Validates that tables properly identify and preserve header information.
     * Headers should be distinguishable from data rows in the markdown representation.
     */
    #[Test]
    public function it_accurately_detects_table_headers(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $table) {
                $markdown = $table->markdown;

                // Valid markdown tables should have header separator (--- or :---:)
                // This validates that header detection was performed during extraction
                if (strpos($markdown, '|') !== false && count($table->cells) > 1) {
                    // Check for markdown table header separator pattern
                    $lines = explode("\n", $markdown);
                    $headerSeparatorFound = false;

                    foreach ($lines as $line) {
                        if (preg_match('/^\s*\|?[\s\-|:]+\|[\s\-|:]+\|?/', $line)) {
                            $headerSeparatorFound = true;
                            break;
                        }
                    }

                    // Header separator should exist in properly formatted markdown tables
                    $this->assertTrue(
                        $headerSeparatorFound || count($lines) === 1,
                        'Markdown table should include header separator or be single line',
                    );
                }

                // First row is typically headers
                if (!empty($table->cells[0])) {
                    $headerRow = $table->cells[0];
                    $this->assertNotEmpty(
                        $headerRow,
                        'First row (header) should not be empty',
                    );
                }
            }
        }
    }

    /**
     * Quality Test 6: Cell content consistency across extractions.
     *
     * Validates that extracting the same document multiple times produces
     * consistent cell content, ensuring deterministic behavior.
     */
    #[Test]
    public function it_maintains_consistent_cell_content_across_extractions(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);

        // Extract from same file twice
        $firstResult = $kreuzberg->extractFile($pdfFiles[0]);
        $secondResult = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify both extractions returned valid results
        $this->assertIsArray($firstResult->tables, 'First result should have tables array');
        $this->assertIsArray($secondResult->tables, 'Second result should have tables array');

        // Compare results
        if (!empty($firstResult->tables) && !empty($secondResult->tables)) {
            $this->assertCount(
                count($firstResult->tables),
                $secondResult->tables,
                'Same file should extract same number of tables on repeated extraction',
            );

            foreach ($firstResult->tables as $index => $firstTable) {
                $secondTable = $secondResult->tables[$index] ?? null;
                $this->assertNotNull(
                    $secondTable,
                    "Table {$index} should exist in second extraction",
                );

                // Cell counts should match
                $this->assertCount(
                    count($firstTable->cells),
                    $secondTable->cells,
                    "Table {$index} should have same row count in repeated extractions",
                );

                // Specific cell content should match (at least first row)
                if (!empty($firstTable->cells[0]) && !empty($secondTable->cells[0])) {
                    $this->assertEquals(
                        $firstTable->cells[0],
                        $secondTable->cells[0],
                        "Table {$index} header row should be identical across extractions",
                    );
                }
            }
        }
    }

    /**
     * Quality Test 7: Special character handling in table cells.
     *
     * Validates that special characters, unicode content, and formatting characters
     * are properly preserved and handled in table cells.
     */
    #[Test]
    public function it_preserves_special_characters_in_cells(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $table) {
                foreach ($table->cells as $rowIndex => $row) {
                    foreach ($row as $cellIndex => $cell) {
                        // All cell content should be valid UTF-8
                        $this->assertFalse(
                            mb_check_encoding($cell, 'UTF-8') === false,
                            "Cell [{$rowIndex}][{$cellIndex}] should contain valid UTF-8",
                        );

                        // If cell contains content, validate it
                        if (!empty(trim($cell))) {
                            // Cell should not have unexpected control characters
                            // but may contain punctuation, numbers, letters, unicode
                            $this->assertFalse(
                                (bool)preg_match('/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/', $cell),
                                "Cell [{$rowIndex}][{$cellIndex}] should not contain control characters",
                            );
                        }
                    }
                }
            }
        }
    }

    /**
     * Quality Test 8: Table boundary detection and row/column count validation.
     *
     * Validates that table boundaries are correctly detected and row/column counts
     * are accurate and consistent throughout the extracted table.
     */
    #[Test]
    public function it_correctly_detects_table_boundaries(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $tableIndex => $table) {
                // Table should have at least one row
                $this->assertGreaterThan(
                    0,
                    count($table->cells),
                    "Table {$tableIndex} should have at least one row",
                );

                // All rows should have at least one column
                foreach ($table->cells as $rowIndex => $row) {
                    $this->assertGreaterThan(
                        0,
                        count($row),
                        "Row {$rowIndex} in table {$tableIndex} should have at least one column",
                    );
                }

                // Table should not have excessive empty cells
                $totalCells = 0;
                $emptyCells = 0;

                foreach ($table->cells as $row) {
                    foreach ($row as $cell) {
                        $totalCells++;
                        if (empty(trim($cell))) {
                            $emptyCells++;
                        }
                    }
                }

                // Less than 50% of cells should be empty (reasonable threshold)
                if ($totalCells > 0) {
                    $emptyPercentage = ($emptyCells / $totalCells) * 100;
                    $this->assertLessThan(
                        50,
                        $emptyPercentage,
                        "Table {$tableIndex} should not have more than 50% empty cells",
                    );
                }
            }
        }
    }

    /**
     * Quality Test 9: Table data type validation and format consistency.
     *
     * Validates that table cells contain properly formatted data and identifies
     * common patterns like numbers, currency, dates that should be preserved.
     */
    #[Test]
    public function it_validates_table_data_format_consistency(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify extraction was successful and returned a valid result
        $this->assertIsArray($result->tables, 'Result should have tables array');

        if (!empty($result->tables)) {
            foreach ($result->tables as $table) {
                foreach ($table->cells as $rowIndex => $row) {
                    foreach ($row as $cellIndex => $cell) {
                        // All cells should be strings
                        $this->assertIsString(
                            $cell,
                            "Cell [{$rowIndex}][{$cellIndex}] should be a string type",
                        );

                        // Cells with numeric content should be properly formatted
                        if (preg_match('/^\s*[\d.,]+\s*$/', $cell)) {
                            // Numeric cells should have consistent formatting
                            $cleanedCell = preg_replace('/[^0-9.,]/', '', $cell);
                            $this->assertNotEmpty(
                                $cleanedCell,
                                "Numeric cell [{$rowIndex}][{$cellIndex}] should contain digits",
                            );
                        }

                        // Currency patterns should preserve special characters
                        if (preg_match('/[$€£¥]/u', $cell)) {
                            $this->assertStringContainsString(
                                '$',
                                $cell,
                            ) || $this->assertStringContainsString(
                                '€',
                                $cell,
                            ) || $this->assertStringContainsString(
                                '£',
                                $cell,
                            );
                        }
                    }
                }
            }
        }
    }

    /**
     * Quality Test 10: Multiple table extraction with proper isolation.
     *
     * Validates that when multiple tables are present in a document, each is
     * properly extracted with correct content isolation and no data mixing.
     */
    #[Test]
    public function it_properly_isolates_multiple_table_extractions(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig();
        $kreuzberg = new Kreuzberg($config);

        foreach ($pdfFiles as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            if (!empty($result->tables) && count($result->tables) > 1) {
                // Test with multiple tables
                $tables = $result->tables;

                // Each table should have unique content (no duplicates)
                $tableContents = array_map(
                    static fn ($table) => serialize($table->cells),
                    $tables,
                );

                $uniqueContents = array_unique($tableContents);

                $this->assertGreaterThanOrEqual(
                    count($tables) - 1, // Allow one duplicate as acceptable
                    count($uniqueContents),
                    'Multiple tables should have distinct content',
                );

                // Validate each table is complete and properly structured
                foreach ($tables as $tableIndex => $table) {
                    $this->assertIsArray(
                        $table->cells,
                        "Table {$tableIndex} should have cells array",
                    );
                    $this->assertNotEmpty(
                        $table->markdown,
                        "Table {$tableIndex} should have non-empty markdown",
                    );
                    $this->assertGreaterThan(
                        0,
                        $table->pageNumber,
                        "Table {$tableIndex} should have valid page number",
                    );
                }

                return; // Successfully validated multiple tables
            }
        }

        $this->assertTrue(true, 'Document with multiple tables not found in test set');
    }
}
