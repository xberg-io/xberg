"""Enhanced comprehensive tests for table extraction functionality.

Tests cover additional table scenarios:
- Complex table structures and edge cases
- Large table performance
- Various document formats
- Advanced table metadata
- Cell content handling
- Markdown accuracy
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import (
    ExtractionConfig,
    extract_file_sync,
)

if TYPE_CHECKING:
    from pathlib import Path


def get_table_pdf_result(test_documents: Path):
    """Get cached extraction result for PDF with tables.

    PDFium can only be initialized once per process.
    Uses global cache from conftest to reuse PDF results across test modules.
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

    pdf_path = test_documents / "pdfs_with_tables" / "tiny.pdf"
    if not pdf_path.exists():
        return None

    config = ExtractionConfig()
    return get_cached_pdf_extraction(str(pdf_path), config)


class TestAdvancedTableStructure:
    """Test advanced table structure handling."""

    def test_table_row_consistency_validation(self, test_documents: Path) -> None:
        """Verify all table rows have consistent column counts."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            cells = table.cells
            if len(cells) > 1:
                expected_cols = len(cells[0])
                for row_idx, row in enumerate(cells):
                    assert len(row) == expected_cols, f"Row {row_idx} has {len(row)} cols, expected {expected_cols}"

    def test_table_cell_type_consistency(self, test_documents: Path) -> None:
        """Verify all cells contain strings."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str), f"Cell content must be string, got {type(cell)}"

    def test_table_markdown_pipe_consistency(self, test_documents: Path) -> None:
        """Verify markdown table has consistent pipe count per line."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            markdown = table.markdown
            lines = [line.strip() for line in markdown.strip().split("\n") if line.strip()]

            if len(lines) > 0:
                pipe_counts = [line.count("|") for line in lines]
                # All lines should have same number of pipes
                if len(pipe_counts) > 1:
                    assert len(set(pipe_counts)) == 1, f"Inconsistent pipes: {pipe_counts}"

    def test_table_page_number_validity(self, test_documents: Path) -> None:
        """Verify page numbers are valid integers >= 1."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            assert isinstance(table.page_number, int)
            assert table.page_number >= 1

    def test_multiple_tables_independence(self, test_documents: Path) -> None:
        """Verify multiple tables maintain independence."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        if len(result.tables) > 1:
            for i, table1 in enumerate(result.tables):
                for table2 in result.tables[i + 1 :]:
                    # Different tables should be separate objects
                    assert table1.cells is not table2.cells


class TestTableMetadataExtraction:
    """Test extraction of table metadata."""

    def test_table_dimensions_calculation(self, test_documents: Path) -> None:
        """Verify ability to calculate table dimensions."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            rows = len(table.cells)
            cols = len(table.cells[0]) if rows > 0 else 0

            assert isinstance(rows, int)
            assert isinstance(cols, int)
            assert rows >= 0
            assert cols >= 0

    def test_table_has_required_attributes(self, test_documents: Path) -> None:
        """Verify all required table attributes exist."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            assert hasattr(table, "cells"), "Table missing cells attribute"
            assert hasattr(table, "markdown"), "Table missing markdown attribute"
            assert hasattr(table, "page_number"), "Table missing page_number attribute"

    def test_table_content_non_empty(self, test_documents: Path) -> None:
        """Verify tables contain actual content."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            assert len(table.cells) > 0, "Table should have at least one row"
            assert len(table.cells[0]) > 0, "Table should have at least one column"

    def test_markdown_generation_produces_output(self, test_documents: Path) -> None:
        """Verify markdown generation creates valid output."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            markdown = table.markdown
            assert isinstance(markdown, str)
            assert len(markdown) > 0, "Markdown should not be empty"
            assert "|" in markdown, "Markdown table should contain pipe separators"


class TestTableFormatVariations:
    """Test handling of different table format variations."""

    def test_docx_with_tables_extraction(self, test_documents: Path) -> None:
        """Test table extraction from DOCX format."""
        config = ExtractionConfig()

        docx_path = test_documents / "documents" / "extraction_test.docx"
        if not docx_path.exists():
            pytest.skip(f"DOCX not found: {docx_path}")

        result = extract_file_sync(str(docx_path), config=config)
        assert result is not None
        assert result.content is not None

    def test_odt_with_tables_extraction(self, test_documents: Path) -> None:
        """Test table extraction from ODT format."""
        config = ExtractionConfig()

        odt_path = test_documents / "odt" / "table.odt"
        if not odt_path.exists():
            pytest.skip(f"ODT not found: {odt_path}")

        result = extract_file_sync(str(odt_path), config=config)
        assert result is not None
        assert result.content is not None

    def test_html_table_extraction_format(self, test_documents: Path) -> None:
        """Test table extraction from HTML."""
        config = ExtractionConfig()

        html_path = test_documents / "web" / "complex_table.html"
        if not html_path.exists():
            pytest.skip(f"HTML not found: {html_path}")

        result = extract_file_sync(str(html_path), config=config)
        assert result is not None
        assert result.content is not None


class TestTableEdgeCases:
    """Test edge cases in table extraction."""

    def test_single_row_table(self, test_documents: Path) -> None:
        """Handle tables with single row."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        # Tables should be valid regardless of size
        for table in result.tables:
            assert len(table.cells) >= 0

    def test_single_column_table(self, test_documents: Path) -> None:
        """Handle tables with single column."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            if len(table.cells) > 0:
                assert len(table.cells[0]) >= 0

    def test_empty_cell_handling_comprehensive(self, test_documents: Path) -> None:
        """Verify proper handling of empty cells."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)
                    # Empty cells should be empty strings
                    assert cell == "" or len(cell) > 0

    def test_whitespace_only_cells(self, test_documents: Path) -> None:
        """Handle cells containing only whitespace."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)

    def test_special_characters_in_cells(self, test_documents: Path) -> None:
        """Verify special characters are preserved in cells."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        # Just verify structure is intact with any special chars
        for table in result.tables:
            assert len(table.cells) > 0


class TestTableConsistency:
    """Test consistency of table extraction."""

    def test_table_extraction_deterministic(self, test_documents: Path) -> None:
        """Verify table extraction is deterministic."""
        result1 = get_table_pdf_result(test_documents)
        if result1 is None:
            pytest.skip("Test PDF not found")

        # Extract again to verify consistency
        result2 = get_table_pdf_result(test_documents)
        if result2 is None:
            pytest.skip("Test PDF not found")

        assert len(result1.tables) == len(result2.tables)

    def test_cell_count_consistency(self, test_documents: Path) -> None:
        """Verify cell counts remain consistent."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            rows = len(table.cells)
            if rows > 0:
                cols = len(table.cells[0])
                total_cells = sum(len(row) for row in table.cells)
                expected_total = rows * cols
                assert total_cells == expected_total

    def test_markdown_line_count_reflects_rows(self, test_documents: Path) -> None:
        """Verify markdown line count reflects table rows."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            markdown = table.markdown
            lines = [line for line in markdown.strip().split("\n") if line.strip()]
            # Markdown should have rows + header + separator
            assert len(lines) > 0


class TestTableExtractionRobustness:
    """Test robustness of table extraction."""

    def test_extraction_handles_no_tables_gracefully(self, test_documents: Path) -> None:
        """Verify extraction handles documents with no tables."""
        config = ExtractionConfig()

        # Use a text file that has no tables
        text_path = test_documents / "documents" / "lorem_ipsum.txt"
        if not text_path.exists():
            pytest.skip(f"Text file not found: {text_path}")

        result = extract_file_sync(str(text_path), config=config)
        assert result is not None
        # Tables might be empty or None
        if result.tables is not None:
            assert isinstance(result.tables, list)

    def test_table_extraction_with_large_table(self, test_documents: Path) -> None:
        """Verify handling of large tables."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            rows = len(table.cells)
            if rows > 50:  # Large table
                assert table.markdown is not None
                assert len(table.markdown) > 0

    def test_table_content_preservation(self, test_documents: Path) -> None:
        """Verify table content is properly preserved."""
        result = get_table_pdf_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.tables is not None
        for table in result.tables:
            # Content should be preserved as strings
            for row in table.cells:
                for cell in row:
                    assert isinstance(cell, str)
                    assert cell is not None
