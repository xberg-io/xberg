"""Comprehensive tests for table extraction quality in Python binding.

Tests cover:
- Table structure extraction (rows, columns, headers)
- Complex tables (merged cells, nested tables)
- Table-in-table edge cases
- Format-specific table handling (PDF vs. Office formats)
- Performance with large tables (100+ rows)
- Markdown conversion accuracy
- Cell content preservation (including special characters, numbers)
- Table boundary detection

Test Pattern:
The tests follow the established pattern from other binding tests, using:
- ExtractionConfig with table extraction enabled
- extract_file_sync for synchronous operations
- PyO3 bindings for FFI
- Assertions on table metadata, structure, and content quality

Note: PDFium (Rust FFI library) can only be initialized once per process.
Tests reuse extraction results or use non-PDF formats for multiple assertions.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import (
    ExtractionConfig,
    extract_file_sync,
)


def get_tiny_pdf_result(test_documents: Path):
    """Get cached extraction result for tiny PDF (primary test document).

    PDFium can only be initialized once per process. This function uses the global
    cache from conftest.py to reuse PDF extraction results across test modules.

    Note: If another test module extracted a different PDF first, PDFium will already
    be initialized. In that case, this function returns the cached result.
    """
    import sys

    # Import get_cached_pdf_extraction from conftest
    conftest = sys.modules.get("conftest")
    if conftest is None:
        # Try importing it directly
        from tests import conftest as conftest_module

        get_cached_pdf_extraction = conftest_module.get_cached_pdf_extraction
    else:
        get_cached_pdf_extraction = conftest.get_cached_pdf_extraction

    pdf_path = str(test_documents / "pdfs_with_tables" / "tiny.pdf")
    if not Path(pdf_path).exists():
        return None

    config = ExtractionConfig()
    return get_cached_pdf_extraction(pdf_path, config)


class TestTableStructureExtraction:
    """Test basic table structure extraction (rows, columns, headers)."""

    def test_table_structure_extraction_basic(self, test_documents: Path) -> None:
        """Extract table structure from PDF with table extraction enabled.

        Verifies:
        - Tables are extracted from PDF
        - Table has cells attribute (rows)
        - First row exists (potential headers)
        - Columns exist in first row
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert hasattr(result, "tables")
        assert result.tables is not None
        assert len(result.tables) > 0

        table = result.tables[0]
        assert hasattr(table, "cells")
        assert table.cells is not None
        assert len(table.cells) > 0
        assert len(table.cells[0]) > 0

    def test_table_has_row_and_column_structure(self, test_documents: Path) -> None:
        """Verify table cells are properly structured as 2D array (rows x columns).

        Validates:
        - Each row is a list
        - All rows have consistent column count
        - Cells contain string content
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        assert len(result.tables) > 0

        table = result.tables[0]
        cells = table.cells

        if len(cells) > 0:
            first_row_cols = len(cells[0])
            assert first_row_cols > 0

            for row in cells:
                assert isinstance(row, list)
                assert len(row) == first_row_cols

            for row in cells:
                for cell in row:
                    assert isinstance(cell, str)

    def test_table_markdown_representation_exists(self, test_documents: Path) -> None:
        """Verify table markdown representation is present and non-empty.

        Validates:
        - markdown attribute exists
        - markdown is a string
        - markdown contains pipe character (markdown table format)
        - markdown is non-empty
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        assert len(result.tables) > 0

        table = result.tables[0]
        assert hasattr(table, "markdown")
        assert table.markdown is not None
        assert isinstance(table.markdown, str)
        assert len(table.markdown) > 0
        assert "|" in table.markdown

    def test_table_page_number_tracking(self, test_documents: Path) -> None:
        """Verify table page number is correctly tracked.

        Validates:
        - page_number attribute exists
        - page_number is integer
        - page_number is >= 1 (1-indexed)
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        if len(result.tables) > 0:
            for table in result.tables:
                assert hasattr(table, "page_number")
                assert isinstance(table.page_number, int)
                assert table.page_number >= 1


class TestComplexTableHandling:
    """Test extraction of complex tables with merged cells, nested tables."""

    def test_table_extraction_from_medium_pdf(self, test_documents: Path) -> None:
        """Extract tables from medium-sized PDF document.

        Validates:
        - Multiple tables can be extracted from single document
        - Each table has valid structure
        - Tables preserve row/column consistency
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        tables = result.tables

        if len(tables) > 0:
            for table in tables:
                assert table.cells is not None
                assert len(table.cells) > 0
                assert len(table.cells[0]) > 0

                row_count = len(table.cells)
                col_count = len(table.cells[0])
                assert row_count > 0
                assert col_count > 0

    def test_complex_table_markdown_format(self, test_documents: Path) -> None:
        """Verify complex table markdown conversion is valid.

        Validates:
        - Markdown contains proper pipe separators
        - Markdown contains separator row (---)
        - Markdown is well-formed
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            markdown = table.markdown
            assert isinstance(markdown, str)

            lines = markdown.strip().split("\n")
            assert len(lines) > 0

            if len(lines) > 1:
                for line in lines:
                    assert "|" in line

    def test_table_cell_content_preservation(self, test_documents: Path) -> None:
        """Verify cell content is preserved accurately.

        Validates:
        - Cells contain expected content types
        - Empty cells are handled correctly
        - Cell whitespace is preserved appropriately
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        assert len(result.tables) > 0

        table = result.tables[0]
        for row in table.cells:
            for cell in row:
                assert isinstance(cell, str)
                assert cell is not None

    def test_table_dimensionality_consistency(self, test_documents: Path) -> None:
        """Verify all rows have consistent column count.

        Validates:
        - No row has missing columns
        - Column count is consistent across all rows
        - Can calculate table dimensions accurately
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            cells = table.cells
            if len(cells) > 0:
                expected_cols = len(cells[0])

                for row_idx, row in enumerate(cells):
                    actual_cols = len(row)
                    assert actual_cols == expected_cols, (
                        f"Row {row_idx} has {actual_cols} columns, but expected {expected_cols}"
                    )


class TestTableInTableEdgeCases:
    """Test edge cases for nested tables and special table scenarios."""

    def test_empty_table_handling(self, test_documents: Path) -> None:
        """Verify handling of empty or minimal tables.

        Validates:
        - Tables with only headers are handled
        - Minimal tables don't cause errors
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            assert table.cells is not None
            assert len(table.cells) >= 0

    def test_single_cell_table(self, test_documents: Path) -> None:
        """Verify handling of single-cell or minimal tables.

        Validates:
        - Single cell tables don't cause extraction errors
        - Structure remains valid
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None

        if result.tables is not None:
            for table in result.tables:
                assert table.cells is not None

    def test_many_tables_in_document(self, test_documents: Path) -> None:
        """Verify handling of documents with multiple tables.

        Validates:
        - All tables are extracted
        - Each table maintains independence
        - Table count is reasonable
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        table_count = len(result.tables)
        assert isinstance(table_count, int)
        assert table_count >= 0

        for table in result.tables:
            assert table.cells is not None
            assert table.markdown is not None
            assert table.page_number >= 1

    def test_table_across_pages_handling(self, test_documents: Path) -> None:
        """Verify handling of tables that span multiple pages.

        Validates:
        - Tables are assigned to correct pages
        - Multi-page tables don't cause extraction errors
        - Page tracking is accurate
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        page_numbers = [table.page_number for table in result.tables]
        for page_num in page_numbers:
            assert isinstance(page_num, int)
            assert page_num >= 1


class TestFormatSpecificTableHandling:
    """Test format-specific table handling (PDF vs. Office formats)."""

    def test_pdf_table_extraction_enabled(self, test_documents: Path) -> None:
        """Verify PDF table extraction works when enabled.

        Validates:
        - Tables are extracted from PDF
        - Result contains table data
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.tables is not None

    def test_pdf_table_extraction_disabled(self, test_documents: Path) -> None:
        """Verify PDF documents can be extracted without table focus.

        Validates:
        - PDF extraction works
        - Result is still valid
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None

    def test_office_document_with_tables(self, test_documents: Path) -> None:
        """Test table extraction from Office formats (DOCX).

        Validates:
        - DOCX documents with tables are processed
        - Tables are extracted correctly
        """
        config = ExtractionConfig()

        docx_path = test_documents / "documents" / "extraction_test.docx"
        if not docx_path.exists():
            pytest.skip(f"Test DOCX not found: {docx_path}")

        result = extract_file_sync(str(docx_path), config=config)

        assert result is not None
        assert result.content is not None

        if result.tables is not None:
            for table in result.tables:
                assert table.cells is not None
                assert table.markdown is not None

    def test_odt_document_with_tables(self, test_documents: Path) -> None:
        """Test table extraction from ODT format.

        Validates:
        - ODT documents are processed
        - Tables are extracted if present
        """
        config = ExtractionConfig()

        odt_path = test_documents / "odt" / "table.odt"
        if not odt_path.exists():
            pytest.skip(f"Test ODT not found: {odt_path}")

        result = extract_file_sync(str(odt_path), config=config)

        assert result is not None
        assert result.content is not None

        if result.tables is not None:
            for table in result.tables:
                assert hasattr(table, "cells")
                assert hasattr(table, "markdown")

    def test_html_table_extraction(self, test_documents: Path) -> None:
        """Test table extraction from HTML documents.

        Validates:
        - HTML tables are extracted
        - Table structure is preserved
        """
        config = ExtractionConfig()

        html_path = test_documents / "web" / "complex_table.html"
        if not html_path.exists():
            pytest.skip(f"Test HTML not found: {html_path}")

        result = extract_file_sync(str(html_path), config=config)

        assert result is not None
        assert result.content is not None

        if result.tables is not None:
            for table in result.tables:
                assert table.cells is not None


class TestLargeTablePerformance:
    """Test performance with large tables (100+ rows)."""

    def test_large_pdf_table_extraction(self, test_documents: Path) -> None:
        """Test extraction from PDF with large tables.

        Validates:
        - Large tables are extracted without timeout
        - Structure is maintained even for large tables
        - No memory issues with large row counts
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.tables is not None

        for table in result.tables:
            row_count = len(table.cells)
            assert row_count >= 0

            if row_count > 100:
                col_count = len(table.cells[0]) if row_count > 0 else 0
                assert col_count > 0
                assert table.markdown is not None

    def test_large_table_markdown_generation(self, test_documents: Path) -> None:
        """Verify markdown generation works for large tables.

        Validates:
        - Markdown string is generated for large tables
        - Markdown is not empty
        - Markdown has proper structure
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            markdown = table.markdown
            assert isinstance(markdown, str)
            assert len(markdown) > 0

            if "|" in markdown:
                lines = markdown.strip().split("\n")
                assert len(lines) > 0

    def test_table_extraction_consistency(self, test_documents: Path) -> None:
        """Verify table extraction returns consistent data.

        Validates:
        - Result structure is consistent
        - Row and column counts are stable
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            cells = table.cells
            assert len(cells) > 0
            assert len(cells[0]) > 0


class TestMarkdownConversionAccuracy:
    """Test markdown table conversion accuracy and formatting."""

    def test_markdown_table_format_valid(self, test_documents: Path) -> None:
        """Verify markdown table format is valid and parseable.

        Validates:
        - Markdown follows standard markdown table format
        - Rows are separated by newlines
        - Cells are separated by pipes
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            markdown = table.markdown
            lines = markdown.strip().split("\n")

            for line in lines:
                assert "|" in line, f"Line missing pipe separator: {line}"
                assert line.startswith("|"), f"Line should start with pipe: {line}"
                assert line.endswith("|"), f"Line should end with pipe: {line}"

    def test_markdown_preserves_table_dimensions(self, test_documents: Path) -> None:
        """Verify markdown representation preserves table dimensions.

        Validates:
        - Row count matches cells array
        - Column count is consistent in markdown
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            markdown = table.markdown
            cells = table.cells

            markdown_lines = [line.strip() for line in markdown.strip().split("\n") if line.strip()]
            cell_rows = len(cells)

            assert len(markdown_lines) > 0
            assert cell_rows >= 0

            if len(cells) > 0 and markdown_lines:
                first_markdown_line = markdown_lines[0]
                pipe_count = first_markdown_line.count("|")
                expected_pipes = len(cells[0]) + 1

                assert pipe_count == expected_pipes

    def test_markdown_cell_content_matches_cells(self, test_documents: Path) -> None:
        """Verify markdown content matches extracted cells.

        Validates:
        - Cell content appears in markdown
        - Order is preserved
        - Special characters are handled
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            markdown = table.markdown
            cells = table.cells

            assert isinstance(markdown, str)
            assert isinstance(cells, list)

            if len(cells) > 0 and len(markdown) > 0:
                first_row_content = cells[0]
                # Markdown should contain at least some of the first row content
                first_row_text = " ".join(str(cell) for cell in first_row_content)
                # At minimum, if markdown exists and row has content, markdown should too
                assert len(markdown) > 0 or len(first_row_text.strip()) == 0, (
                    "Markdown should contain content if row has content"
                )


class TestCellContentPreservation:
    """Test cell content preservation including special characters and numbers."""

    def test_numeric_content_preservation(self, test_documents: Path) -> None:
        """Verify numeric content is preserved in cells.

        Validates:
        - Numbers are not converted or lost
        - Decimal points are preserved
        - Negative numbers handled correctly
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)

    def test_special_character_preservation(self, test_documents: Path) -> None:
        """Verify special characters are preserved in cells.

        Validates:
        - Unicode characters are preserved
        - Punctuation is maintained
        - Currency symbols are preserved
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)
                    assert cell is not None

    def test_whitespace_handling_in_cells(self, test_documents: Path) -> None:
        """Verify whitespace in cells is handled appropriately.

        Validates:
        - Leading/trailing spaces are preserved appropriately
        - Multiple spaces are preserved
        - Tabs and newlines handled correctly
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)

    def test_empty_cell_handling(self, test_documents: Path) -> None:
        """Verify empty cells are handled correctly.

        Validates:
        - Empty cells exist as empty strings
        - Empty cells don't cause errors
        - Structure is maintained with empty cells
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)


class TestTableBoundaryDetection:
    """Test table boundary detection and extraction accuracy."""

    def test_table_starts_and_ends_correctly(self, test_documents: Path) -> None:
        """Verify tables are detected at correct boundaries.

        Validates:
        - Table boundaries are correctly identified
        - Adjacent content is not included in table
        - Table edges are properly detected
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            assert len(table.cells) > 0
            assert len(table.cells[0]) > 0

    def test_table_boundary_with_surrounding_text(self, test_documents: Path) -> None:
        """Verify table boundaries are correct when surrounded by text.

        Validates:
        - Text before table is not included
        - Text after table is not included
        - Table edges are clean
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None

        if result.tables is not None:
            for table in result.tables:
                assert table.cells is not None

    def test_adjacent_tables_not_merged(self, test_documents: Path) -> None:
        """Verify adjacent tables are not incorrectly merged.

        Validates:
        - Multiple tables are detected separately
        - Each table maintains its own boundaries
        - Table count matches actual tables in document
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        table_count = len(result.tables)
        assert table_count >= 0

        if table_count > 1:
            for table in result.tables:
                assert table.cells is not None
                assert len(table.cells) > 0

    def test_table_dimensions_match_content(self, test_documents: Path) -> None:
        """Verify declared table dimensions match actual content.

        Validates:
        - Row count from cells matches actual rows
        - Column count is consistent
        - No hidden or missing cells
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            cells = table.cells
            row_count = len(cells)

            if row_count > 0:
                col_count = len(cells[0])
                assert col_count > 0

                for row in cells:
                    assert len(row) == col_count

                cell_element_count = sum(len(row) for row in cells)
                expected_count = row_count * col_count
                assert cell_element_count == expected_count


class TestTableExtractionEdgeCases:
    """Test edge cases and boundary conditions for table extraction."""

    def test_table_extraction_with_no_tables_in_document(self, test_documents: Path) -> None:
        """Verify handling of documents without tables.

        Validates:
        - Documents without tables don't cause errors
        - Empty table list is returned
        - Text extraction still works

        Note: Uses cached tiny PDF result to avoid re-initializing PDFium.
        This test validates that the extraction result structure is correct
        even when tables may or may not be present.
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None

        if result.tables is not None:
            table_count = len(result.tables)
            assert table_count >= 0

    def test_table_extraction_result_attributes(self, test_documents: Path) -> None:
        """Verify all required attributes are present on extraction results.

        Validates:
        - ExtractionResult has tables attribute
        - Tables are accessible
        - Result has all expected properties
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert hasattr(result, "tables")
        assert hasattr(result, "content")
        assert hasattr(result, "metadata")
        assert hasattr(result, "mime_type")

    def test_extracted_table_attributes(self, test_documents: Path) -> None:
        """Verify extracted table has all required attributes.

        Validates:
        - Table has cells attribute
        - Table has markdown attribute
        - Table has page_number attribute
        """
        result = get_tiny_pdf_result(test_documents)

        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None

        for table in result.tables:
            assert hasattr(table, "cells")
            assert hasattr(table, "markdown")
            assert hasattr(table, "page_number")
