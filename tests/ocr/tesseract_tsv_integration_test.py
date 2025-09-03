from __future__ import annotations

from pathlib import Path

import pytest
from PIL import Image

from kreuzberg._ocr._table_extractor import (
    detect_columns,
    detect_rows,
    extract_table_from_tsv,
    extract_words,
    reconstruct_table,
    to_markdown,
)
from kreuzberg._ocr._tesseract import TesseractBackend


@pytest.fixture
def table_image_path() -> Path:
    path = Path("tests/test_source_files/tables/simple_table.png")
    if not path.exists():
        pytest.skip(f"Test image not found: {path}")
    return path


@pytest.fixture
def science_table_image() -> Path:
    path = Path("tests/test_source_files/tables/complex_document.png")
    if not path.exists():
        pytest.skip(f"Test image not found: {path}")
    return path


@pytest.mark.anyio
async def test_tesseract_tsv_output_integration(table_image_path: Path) -> None:
    """Test that TSV output format produces extracted text from the image."""
    backend = TesseractBackend()

    result = await backend.process_file(table_image_path, output_format="tsv", enable_table_detection=False)

    assert result is not None
    assert isinstance(result.content, str)
    assert len(result.content) > 0

    assert "Product" in result.content or "product" in result.content.lower(), "Should extract 'Product' text"
    assert "Price" in result.content or "price" in result.content.lower(), "Should extract 'Price' text"

    lines = result.content.strip().split("\n")
    assert len(lines) >= 1, "Should have at least one line of text"


@pytest.mark.anyio
async def test_tesseract_process_image_with_table_detection(table_image_path: Path) -> None:
    """Test table detection from PIL Image with default TSV processing."""
    backend = TesseractBackend()

    with Image.open(table_image_path) as img:
        result = await backend.process_image(
            img,
            enable_table_detection=True,
            table_column_threshold=30,
            table_row_threshold_ratio=0.5,
            table_min_confidence=30.0,
        )

    assert result is not None
    assert result.content, "Should extract text content"

    content_lower = result.content.lower()
    assert "product" in content_lower or "price" in content_lower, "Should extract table text"

    if result.tables:
        assert len(result.tables) > 0, "Should detect at least one table"
        table = result.tables[0]

        assert isinstance(table, dict), "Table should be a dictionary"
        assert "text" in table, "Table should have 'text' key containing markdown"
        assert "metadata" in table, "Table should have 'metadata' key"
        assert "page_number" in table, "Table should have 'page_number' key"

        if table["text"]:
            table_text = table["text"]
            assert isinstance(table_text, str), "Table text should be string"
            if "|" in table_text:
                lines = [line.strip() for line in table_text.split("\n") if line.strip()]
                assert len(lines) >= 2, "Markdown table needs at least header and separator"


@pytest.mark.anyio
async def test_table_detection_with_tsv_format(table_image_path: Path) -> None:
    """Test table detection when using TSV output format."""
    backend = TesseractBackend()

    result = await backend.process_file(
        table_image_path,
        output_format="tsv",
        enable_table_detection=True,
        table_column_threshold=20,
        table_row_threshold_ratio=0.5,
        table_min_confidence=30.0,
    )

    assert result is not None
    assert result.content, "Should have extracted content"

    if result.tables:
        assert len(result.tables) > 0, "Should detect at least one table"
        table = result.tables[0]

        assert "text" in table, "Table must have 'text' key"
        assert "page_number" in table, "Table should have page number field"

        table_text = table["text"]
        assert isinstance(table_text, str), "Table text should be string"
        assert "|" in table_text, "Table should be in markdown format"

        lines = [line.strip() for line in table_text.split("\n") if line.strip()]
        assert len(lines) >= 3, "Markdown table needs header, separator, and data rows"

        assert "---" in lines[1], "Second line should have markdown separator"

        header_cols = lines[0].count("|")
        assert all(line.count("|") == header_cols for line in lines), "All lines should have same column count"


def test_table_extractor_with_real_tsv() -> None:
    """Test complete table extraction pipeline with real-world TSV data."""
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
1\t1\t0\t0\t0\t0\t0\t0\t800\t600\t-1\t
5\t1\t1\t1\t1\t1\t100\t100\t80\t30\t95.0\tProduct
5\t1\t1\t1\t1\t2\t250\t100\t60\t30\t94.0\tPrice
5\t1\t1\t1\t1\t3\t400\t100\t80\t30\t96.0\tQuantity
5\t1\t2\t1\t1\t1\t100\t150\t80\t30\t92.0\tApples
5\t1\t2\t1\t1\t2\t250\t150\t60\t30\t93.0\t$2.50
5\t1\t2\t1\t1\t3\t400\t150\t40\t30\t91.0\t10
5\t1\t3\t1\t1\t1\t100\t200\t80\t30\t94.0\tBananas
5\t1\t3\t1\t1\t2\t250\t200\t60\t30\t92.0\t$1.20
5\t1\t3\t1\t1\t3\t400\t200\t40\t30\t93.0\t15"""

    words = extract_words(tsv_data, min_confidence=90.0)
    assert len(words) == 9, "Should extract 9 words above 90% confidence"
    assert words[0]["text"] == "Product", "First word should be 'Product'"
    assert words[0]["conf"] == 95.0, "Product confidence should be 95.0"
    assert words[0]["left"] == 100, "Product X position should be 100"
    assert words[0]["top"] == 100, "Product Y position should be 100"

    cols = detect_columns(words, column_threshold=20)
    assert len(cols) == 3, "Should detect 3 columns"
    assert cols[0] == 100, "First column should be at X=100"
    assert cols[1] == 250, "Second column should be at X=250"
    assert cols[2] == 400, "Third column should be at X=400"

    rows = detect_rows(words, row_threshold_ratio=0.5)
    assert len(rows) == 3, "Should detect 3 rows"
    assert rows[0] == 115, "First row center should be at Y=115"
    assert rows[1] == 165, "Second row center should be at Y=165"
    assert rows[2] == 215, "Third row center should be at Y=215"

    table = reconstruct_table(words, column_threshold=20, row_threshold_ratio=0.5)
    assert len(table) == 3, "Reconstructed table should have 3 rows"
    assert table[0] == ["Product", "Price", "Quantity"], "Header row exact match"
    assert table[1] == ["Apples", "$2.50", "10"], "First data row exact match"
    assert table[2] == ["Bananas", "$1.20", "15"], "Second data row exact match"

    markdown = to_markdown(table)
    expected_lines = [
        "| Product | Price | Quantity |",
        "| --- | --- | --- |",
        "| Apples | $2.50 | 10 |",
        "| Bananas | $1.20 | 15 |",
    ]
    actual_lines = markdown.strip().split("\n")
    assert actual_lines == expected_lines, "Markdown output should match exactly"


def test_extract_table_from_tsv_convenience() -> None:
    """Test the all-in-one convenience function for TSV to markdown conversion."""
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t40\t20\t95.0\tA
5\t1\t1\t1\t1\t2\t150\t50\t40\t20\t94.0\tB
5\t1\t2\t1\t1\t1\t50\t100\t40\t20\t93.0\t1
5\t1\t2\t1\t1\t2\t150\t100\t40\t20\t92.0\t2"""

    markdown = extract_table_from_tsv(tsv_data)
    assert markdown != "", "Should produce markdown output"

    expected = """| A | B |
| --- | --- |
| 1 | 2 |"""
    assert markdown == expected, f"Expected:\n{expected}\nGot:\n{markdown}"

    markdown_custom = extract_table_from_tsv(
        tsv_data,
        column_threshold=200,
        row_threshold_ratio=0.5,
        min_confidence=30.0,
    )
    assert markdown_custom != "", "Should produce output even with different parameters"


def test_table_extraction_with_empty_cells() -> None:
    """Test handling of tables with missing/empty cells in the middle."""
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t60\t30\t95.0\tHeader1
5\t1\t1\t1\t1\t2\t200\t50\t60\t30\t94.0\tHeader2
5\t1\t1\t1\t1\t3\t350\t50\t60\t30\t96.0\tHeader3
5\t1\t2\t1\t1\t1\t50\t100\t60\t30\t92.0\tData1
5\t1\t2\t1\t1\t2\t350\t100\t60\t30\t91.0\tData3"""

    words = extract_words(tsv_data, min_confidence=30.0)
    assert len(words) == 5, "Should extract all 5 words"

    cols = detect_columns(words, column_threshold=20)
    assert len(cols) == 3, "Should detect 3 columns despite missing middle cell"

    table = reconstruct_table(words, column_threshold=20, row_threshold_ratio=0.5)
    assert len(table) == 2, "Should have 2 rows"
    assert table[0] == ["Header1", "Header2", "Header3"], "Header row should be complete"
    assert table[1] == ["Data1", "", "Data3"], "Data row should have empty middle cell"

    markdown = to_markdown(table)
    assert "| Data1 |  | Data3 |" in markdown, "Empty cell should be represented as empty string in markdown"


def test_table_extraction_confidence_threshold() -> None:
    """Test that confidence threshold properly filters low-confidence words."""
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t60\t30\t95.0\tGood
5\t1\t1\t1\t1\t2\t150\t50\t60\t30\t20.0\tBad
5\t1\t1\t1\t1\t3\t250\t50\t60\t30\t85.0\tAlsoGood"""

    words_default = extract_words(tsv_data, min_confidence=30.0)
    assert len(words_default) == 2, "Should filter out word with 20% confidence"
    assert words_default[0]["text"] == "Good", "First word should be 'Good'"
    assert words_default[0]["conf"] == 95.0, "Good confidence should be 95.0"
    assert words_default[1]["text"] == "AlsoGood", "Second word should be 'AlsoGood'"
    assert words_default[1]["conf"] == 85.0, "AlsoGood confidence should be 85.0"

    words_high = extract_words(tsv_data, min_confidence=90.0)
    assert len(words_high) == 1, "Only one word above 90% confidence"
    assert words_high[0]["text"] == "Good", "Only 'Good' should pass 90% threshold"

    words_low = extract_words(tsv_data, min_confidence=10.0)
    assert len(words_low) == 3, "All words should pass 10% threshold"
    assert [w["text"] for w in words_low] == ["Good", "Bad", "AlsoGood"], "All words in order"


@pytest.mark.parametrize(
    "column_threshold,expected_cols,expected_positions",
    [
        (10, 3, [50, 80, 200]),
        (50, 2, [65, 200]),
        (200, 1, [80]),
    ],
)
def test_column_clustering_thresholds(column_threshold: int, expected_cols: int, expected_positions: list[int]) -> None:
    """Test column detection with various clustering thresholds."""
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t40\t30\t95.0\tA
5\t1\t1\t1\t1\t2\t80\t50\t40\t30\t94.0\tB
5\t1\t1\t1\t1\t3\t200\t50\t40\t30\t93.0\tC"""

    words = extract_words(tsv_data, min_confidence=30.0)
    assert len(words) == 3, "Should extract all 3 words"

    cols = detect_columns(words, column_threshold=column_threshold)
    assert len(cols) == expected_cols, f"Should detect {expected_cols} columns with threshold {column_threshold}"
    assert cols == expected_positions, f"Column positions should be {expected_positions}"

    table = reconstruct_table(words, column_threshold=column_threshold, row_threshold_ratio=0.5)
    if expected_cols == 3:
        assert table == [["A", "B", "C"]], "Three separate columns"
    elif expected_cols == 2:
        assert len(table) == 1, "Should have one row"
        assert len(table[0]) == 2, "Should have two columns"
        assert "A B" in table[0][0] or "B" in table[0][0], "First two words should merge"
    else:
        assert len(table) == 1, "Should have one row"
        assert len(table[0]) == 1, "Should have one column"
        assert "A B C" in table[0][0] or "C" in table[0][0], "All words should merge"
