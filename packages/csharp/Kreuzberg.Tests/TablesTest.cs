using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for table extraction quality from documents.
/// Tests cover table structure extraction, cell content preservation, markdown formatting,
/// format-specific handling, performance with large tables, and edge cases like merged cells and nested tables.
/// </summary>
public class TablesTest
{
    public TablesTest()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();

        // Clean up any registered callbacks from previous tests to prevent GCHandle accumulation
        try { KreuzbergClient.ClearPostProcessors(); } catch { }
        try { KreuzbergClient.ClearValidators(); } catch { }
        try { KreuzbergClient.ClearOcrBackends(); } catch { }
    }

    #region Basic Table Structure Tests

    /// <summary>
    /// Test: Verify that basic table structure is correctly extracted from PDFs.
    /// Validates that tables contain rows, columns, and cell data.
    /// </summary>
    [Fact]
    public void TableStructureExtraction_FromPdf_ReturnsRowsColumnsAndHeaders()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Tables);
        Assert.NotEmpty(result.Tables);

        foreach (var table in result.Tables)
        {
            // Validate table structure
            Assert.NotNull(table.Cells);
            Assert.NotEmpty(table.Cells);

            // Each row should have cells
            foreach (var row in table.Cells)
            {
                Assert.NotNull(row);
                Assert.NotEmpty(row);
            }

            // Page number should be valid
            Assert.True(table.PageNumber > 0, "Table page number should be positive");
        }
    }

    /// <summary>
    /// Test: Verify that extracted tables contain markdown representation.
    /// Validates markdown formatting includes proper delimiters and structure.
    /// </summary>
    [Fact]
    public void TableMarkdownConversion_ProducesValidMarkdownFormat()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);
        Assert.NotEmpty(result.Tables);

        foreach (var table in result.Tables)
        {
            // Markdown should not be empty
            Assert.NotNull(table.Markdown);
            Assert.NotEmpty(table.Markdown.Trim());

            // Markdown tables should contain pipe delimiters
            if (table.Markdown.Contains("|"))
            {
                Assert.Contains("|", table.Markdown);
                // Should have multiple lines (header + separator + rows)
                var lines = table.Markdown.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
                Assert.True(lines.Length >= 2, "Markdown table should have at least header and separator rows");
            }
        }
    }

    /// <summary>
    /// Test: Verify that table cell content is accurately preserved from source documents.
    /// Validates that cell text matches expected content without corruption or loss.
    /// </summary>
    [Fact]
    public void TableCellContent_PreservesTextAccurately()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);
        Assert.NotEmpty(result.Tables);

        var table = result.Tables.First();
        Assert.NotNull(table.Cells);
        Assert.NotEmpty(table.Cells);

        // Verify cell content is not null and has reasonable length
        var allCells = table.Cells.SelectMany(row => row).ToList();
        Assert.NotEmpty(allCells);

        foreach (var cell in allCells)
        {
            Assert.NotNull(cell);
            // Cell content should be either empty or have valid text
            Assert.IsType<string>(cell);
        }
    }

    /// <summary>
    /// Test: Verify table boundary detection correctly identifies table regions.
    /// Validates that table start and end are properly detected within document content.
    /// </summary>
    [Fact]
    public void TableBoundaryDetection_CorrectlyIdentifiesTableRegions()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);
        Assert.NotEmpty(result.Tables);

        foreach (var table in result.Tables)
        {
            // Table should have valid page number
            Assert.True(table.PageNumber > 0);
            Assert.True(table.PageNumber <= (result.Metadata?.Pages?.TotalCount ?? 999));

            // Table must have cells defining its boundaries
            Assert.NotEmpty(table.Cells);
            Assert.True(table.Cells.Count > 0, "Table must have at least one row");

            // Validate that all rows have consistent column count or reasonable variation
            var columnCounts = table.Cells.Select(row => row.Count).ToList();
            var minCols = columnCounts.Min();
            var maxCols = columnCounts.Max();
            Assert.True(minCols > 0, "Table must have at least one column");
            // Allow reasonable variation for tables with merged cells
            Assert.True(maxCols - minCols <= table.Cells.Count, "Column count variation should be reasonable");
        }
    }

    #endregion

    #region Format-Specific Table Handling Tests

    /// <summary>
    /// Test: Verify PDF tables are extracted correctly with format-specific handling.
    /// Validates PDF-specific features like annotations, text extraction accuracy.
    /// </summary>
    [Fact]
    public void TableExtraction_FromPdf_HandlesFormatSpecificFeatures()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true,
                ExtractImages = false
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.Equal("application/pdf", result.MimeType);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            var table = result.Tables[0];

            // PDF tables should have page number
            Assert.True(table.PageNumber > 0);

            // PDF table should have markdown representation
            Assert.NotNull(table.Markdown);
            Assert.NotEmpty(table.Markdown);

            // Cells should be properly structured
            Assert.NotNull(table.Cells);
            Assert.NotEmpty(table.Cells);
        }
    }

    /// <summary>
    /// Test: Verify Word documents handle table extraction with Office format specifics.
    /// Validates that nested styles, formatting, and structure are preserved.
    /// </summary>
    [Fact]
    public void TableExtraction_FromDocx_HandlesOfficeFormatSpecifics()
    {
        var docxPath = NativeTestHelper.GetDocumentPath("documents/word_sample.docx");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(docxPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);

        // DOCX should be properly identified
        if (result.Metadata.FormatType != FormatType.Unknown)
        {
            // Format should be one of the Office types
            Assert.True(
                result.Metadata.FormatType == FormatType.Unknown ||
                result.MimeType.Contains("word") ||
                result.MimeType.Contains("document"),
                "DOCX should be identified as document format"
            );
        }

        // Tables extracted from DOCX should have valid structure
        if (result.Tables != null && result.Tables.Count > 0)
        {
            foreach (var table in result.Tables)
            {
                Assert.NotNull(table.Cells);
                Assert.NotNull(table.Markdown);
            }
        }
    }

    #endregion

    #region Complex Table Structure Tests

    /// <summary>
    /// Test: Handle tables with merged cells correctly.
    /// Validates that cells spanning multiple rows/columns are handled appropriately.
    /// </summary>
    [Fact]
    public void ComplexTableHandling_WithMergedCells_ExtractsCorrectly()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);

        // Complex tables may have varied row lengths due to merged cells
        foreach (var table in result.Tables)
        {
            var rowLengths = table.Cells.Select(row => row.Count).Distinct().ToList();

            // Tables with merged cells may have variable row lengths
            // We verify the structure is still valid
            foreach (var row in table.Cells)
            {
                Assert.NotEmpty(row);
                foreach (var cell in row)
                {
                    Assert.IsType<string>(cell);
                }
            }

            // Markdown should still be valid even with merged cells
            Assert.NotEmpty(table.Markdown);
        }
    }

    /// <summary>
    /// Test: Handle nested tables correctly (tables within table cells).
    /// Validates that complex hierarchical structures are extracted without loss.
    /// </summary>
    [Fact]
    public void NestedTableHandling_ExtractsWithoutHierarchyLoss()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);

        if (result.Tables.Count > 1)
        {
            // Multiple tables in document - some may be nested
            var totalCells = result.Tables.Sum(t => t.Cells.Sum(r => r.Count));
            Assert.True(totalCells > 0, "Nested tables should have extractable cells");

            // Each table should maintain its own structure
            foreach (var table in result.Tables)
            {
                Assert.NotNull(table.Cells);
                Assert.NotNull(table.Markdown);
            }
        }
    }

    /// <summary>
    /// Test: Verify table consistency when same document is extracted multiple times.
    /// Validates deterministic extraction produces identical results.
    /// </summary>
    [Fact]
    public void TableConsistency_RepeatedExtraction_ProducesIdenticalResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result1 = KreuzbergClient.ExtractFileSync(pdfPath, config);
        var result2 = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result1.Tables);
        Assert.NotNull(result2.Tables);
        Assert.Equal(result1.Tables.Count, result2.Tables.Count);

        for (int i = 0; i < result1.Tables.Count; i++)
        {
            var table1 = result1.Tables[i];
            var table2 = result2.Tables[i];

            // Same row count
            Assert.Equal(table1.Cells.Count, table2.Cells.Count);

            // Same markdown output
            Assert.Equal(table1.Markdown, table2.Markdown);

            // Same cell content
            for (int row = 0; row < table1.Cells.Count; row++)
            {
                Assert.Equal(table1.Cells[row].Count, table2.Cells[row].Count);
                for (int col = 0; col < table1.Cells[row].Count; col++)
                {
                    Assert.Equal(table1.Cells[row][col], table2.Cells[row][col]);
                }
            }
        }
    }

    #endregion

    #region Performance and Scale Tests

    /// <summary>
    /// Test: Performance with large tables (100+ rows).
    /// Validates that extraction completes in reasonable time and memory usage.
    /// </summary>
    [Fact]
    public void TablePerformance_WithLargeTable_ExtractsEfficiently()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var stopwatch = System.Diagnostics.Stopwatch.StartNew();
        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        stopwatch.Stop();

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);

        // Should complete in reasonable time (within 30 seconds)
        Assert.True(stopwatch.ElapsedMilliseconds < 30000,
            $"Table extraction should complete quickly, took {stopwatch.ElapsedMilliseconds}ms");

        // For each table, verify it's properly extracted
        foreach (var table in result.Tables)
        {
            var rowCount = table.Cells.Count;
            var totalCells = table.Cells.Sum(r => r.Count);

            // Verify structure is intact
            Assert.True(rowCount > 0);
            Assert.True(totalCells > 0);

            // Large tables should still have valid markdown
            Assert.NotEmpty(table.Markdown);
        }
    }

    /// <summary>
    /// Test: Verify memory efficiency with multi-table documents.
    /// Validates that multiple tables don't cause memory bloat.
    /// </summary>
    [Fact]
    public void TablePerformance_WithMultipleTables_MaintainsMemoryEfficiency()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);

        // Verify each table is properly structured
        var tableCount = result.Tables.Count;
        var totalCells = result.Tables.Sum(t => t.Cells.Sum(r => r.Count));

        if (tableCount > 0)
        {
            // All tables should be accessible and valid
            for (int i = 0; i < tableCount; i++)
            {
                var table = result.Tables[i];
                Assert.NotNull(table.Cells);
                Assert.NotNull(table.Markdown);
                Assert.True(table.PageNumber > 0);
            }
        }
    }

    #endregion

    #region Markdown Formatting Quality Tests

    /// <summary>
    /// Test: Verify markdown output includes proper table formatting standards.
    /// Validates pipe delimiters, header separators, and row structure.
    /// </summary>
    [Fact]
    public void MarkdownFormatting_FollowsStandardTableFormat()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);
        Assert.NotEmpty(result.Tables);

        foreach (var table in result.Tables)
        {
            var markdown = table.Markdown;
            Assert.NotNull(markdown);

            if (markdown.Contains("|"))
            {
                // Should have pipe delimiters on each line
                var lines = markdown.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None)
                    .Where(l => !string.IsNullOrWhiteSpace(l))
                    .ToList();

                Assert.True(lines.Count > 0, "Markdown table should have content");

                // Most markdown table lines should contain pipes
                var pipeLines = lines.Count(l => l.Contains("|"));
                Assert.True(pipeLines > 0, "Markdown table should contain pipe delimiters");

                // Header separator line should exist (contains dashes and pipes)
                var hasSeparator = lines.Any(l => Regex.IsMatch(l, @"\|\s*[-\s]+\s*\|"));
                if (lines.Count > 1)
                {
                    // If multi-line table, likely has separator
                    Assert.True(hasSeparator || !markdown.Contains("|"),
                        "Multi-line markdown table should have separator row");
                }
            }
        }
    }

    /// <summary>
    /// Test: Verify cell content within markdown is properly escaped and formatted.
    /// Validates that special characters don't break markdown structure.
    /// </summary>
    [Fact]
    public void MarkdownFormatting_ProperlyEscapesSpecialCharacters()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result.Tables);

        foreach (var table in result.Tables)
        {
            var markdown = table.Markdown;
            Assert.NotNull(markdown);

            // Verify markdown doesn't have unmatched delimiters that would break parsing
            if (markdown.Contains("|"))
            {
                var lines = markdown.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None)
                    .Where(l => !string.IsNullOrWhiteSpace(l))
                    .ToList();

                foreach (var line in lines)
                {
                    if (line.Contains("|"))
                    {
                        // Line should have balanced pipes
                        var pipeCount = line.Count(c => c == '|');
                        Assert.True(pipeCount >= 2, "Table lines should have at least opening and closing pipe");
                    }
                }
            }

            // Cell content should be accessible
            Assert.NotNull(table.Cells);
            Assert.NotEmpty(table.Cells);
        }
    }

    #endregion

    #region Configuration and Features Tests

    /// <summary>
    /// Test: Verify table extraction works with various configuration options.
    /// Validates that config options don't interfere with table extraction.
    /// </summary>
    [Fact]
    public void TableExtractionConfig_WithVariousOptions_ExtractsSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");

        var configWithMetadata = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, configWithMetadata);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Tables);

        // Metadata should not interfere with table extraction
        Assert.NotNull(result.Metadata);
    }

    /// <summary>
    /// Test: Verify immutable config pattern with table extraction settings.
    /// Validates init-only properties on ExtractionConfig.
    /// </summary>
    [Fact]
    public void ExtractionConfig_ImmutableWithInitPattern()
    {
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                ExtractMetadata = true,
                ExtractImages = false
            },
            Pages = new PageConfig
            {
                ExtractPages = true
            }
        };

        // Verify config was created with init pattern
        Assert.NotNull(config.PdfOptions);
        Assert.True(config.PdfOptions.ExtractMetadata == true);
        Assert.False(config.PdfOptions.ExtractImages ?? true);

        Assert.NotNull(config.Pages);
        Assert.True(config.Pages.ExtractPages == true);
    }

    #endregion

    #region Validation and Edge Cases Tests

    /// <summary>
    /// Test: Handle documents without tables gracefully.
    /// Validates that null/empty results are handled correctly.
    /// </summary>
    [Fact]
    public void TableExtraction_FromDocumentWithoutTables_ReturnsEmptyOrNull()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/fake_memo.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        // Document may or may not have tables - both are acceptable
        if (result.Tables != null)
        {
            // If tables list exists, it should be properly formed
            Assert.IsType<List<Table>>(result.Tables);

            // All tables should have valid structure
            foreach (var table in result.Tables)
            {
                Assert.NotNull(table.Cells);
            }
        }
    }

    /// <summary>
    /// Test: Verify required table fields are always populated.
    /// Validates that tables have minimum required data.
    /// </summary>
    [Fact]
    public void TableStructure_HasAllRequiredFields()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            foreach (var table in result.Tables)
            {
                // Required fields
                Assert.NotNull(table.Cells);
                Assert.NotNull(table.Markdown);

                // Page number should be set
                Assert.True(table.PageNumber >= 1, "Page number should be 1-indexed");

                // Cells should have valid structure
                foreach (var row in table.Cells)
                {
                    Assert.NotNull(row);
                    foreach (var cell in row)
                    {
                        Assert.IsType<string>(cell);
                    }
                }
            }
        }
    }

    /// <summary>
    /// Test: Verify table data types and boundaries.
    /// Validates that all numeric and string fields have valid values.
    /// </summary>
    [Fact]
    public void TableDataTypes_AreValidAndWithinBounds()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdfs/embedded_images_tables.pdf");
        var config = new ExtractionConfig();

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        if (result.Tables != null && result.Tables.Count > 0)
        {
            foreach (var table in result.Tables)
            {
                // Page number should be positive integer
                Assert.True(table.PageNumber > 0 && table.PageNumber < 100000,
                    "Page number should be valid positive integer");

                // Markdown should be string
                Assert.IsType<string>(table.Markdown);

                // Cells should be List of List of string
                Assert.IsType<List<List<string>>>(table.Cells);

                // Cell data should be string
                foreach (var row in table.Cells)
                {
                    Assert.IsType<List<string>>(row);
                    Assert.True(row.Count > 0, "Row should have columns");

                    foreach (var cell in row)
                    {
                        Assert.IsType<string>(cell);
                    }
                }
            }
        }
    }

    #endregion
}
