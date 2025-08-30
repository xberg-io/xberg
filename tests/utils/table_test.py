"""Tests for table processing utilities."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

import pandas as pd
import pytest

from kreuzberg._utils._table import (
    enhance_table_markdown,
    export_table_to_csv,
    export_table_to_tsv,
    extract_table_structure_info,
    generate_table_summary,
)

if TYPE_CHECKING:
    from kreuzberg._types import TableData


@pytest.fixture
def sample_table_data() -> dict[str, Any]:
    """Create sample table data for testing."""
    df = pd.DataFrame(
        {
            "Name": ["Alice", "Bob", "Charlie"],
            "Age": [25, 30, 35],
            "Score": [95.5, 87.0, 92.3],
            "Active": [True, False, True],
        }
    )
    return {"df": df, "text": "Sample table text", "page_number": 1}


@pytest.fixture
def numeric_table_data() -> dict[str, Any]:
    """Create table with numeric data for testing."""
    df = pd.DataFrame({"Revenue": [1000.50, 2500.75, 1800.25], "Profit": [150, 300, 200], "Margin": [15.0, 12.0, 11.1]})
    return {"df": df, "text": "Financial data", "page_number": 2}


@pytest.fixture
def empty_table_data() -> dict[str, Any]:
    """Create empty table data for testing."""
    return {"df": pd.DataFrame(), "text": "Empty table", "page_number": 3}


@pytest.fixture
def table_with_nulls() -> dict[str, Any]:
    """Create table with null values for testing."""
    df = pd.DataFrame({"Item": ["A", "B", None, "D"], "Value": [10, None, 30, 40], "Category": ["X", "Y", "Z", None]})
    return {"df": df, "text": "Table with nulls", "page_number": 4}


@pytest.fixture
def mixed_type_table() -> dict[str, Any]:
    """Create table with mixed data types for testing."""
    df = pd.DataFrame(
        {
            "ID": ["001", "002", "003"],
            "Amount": ["$1,234.56", "2,500", "3.14"],
            "Percentage": ["15%", "20.5%", "8%"],
            "Text": ["Hello", "World", "Test"],
        }
    )
    return {"df": df, "text": "Mixed type table"}


def test_export_table_to_csv_basic(sample_table_data: dict[str, Any]) -> None:
    """Test basic CSV export functionality."""
    result = export_table_to_csv(cast("TableData", sample_table_data))

    expected_lines = ["Name,Age,Score,Active", "Alice,25,95.5,True", "Bob,30,87.0,False", "Charlie,35,92.3,True"]

    assert result == "\n".join(expected_lines)


def test_export_table_to_csv_custom_separator(sample_table_data: dict[str, Any]) -> None:
    """Test CSV export with custom separator."""
    result = export_table_to_csv(cast("TableData", sample_table_data), separator=";")

    assert "Name;Age;Score;Active" in result
    assert "Alice;25;95.5;True" in result


def test_export_table_to_csv_empty_table(empty_table_data: dict[str, Any]) -> None:
    """Test CSV export with empty table."""
    result = export_table_to_csv(cast("TableData", empty_table_data))

    assert result == ""


def test_export_table_to_csv_no_df() -> None:
    """Test CSV export when df is None."""
    table_data = {"text": "No dataframe", "df": None}
    result = export_table_to_csv(cast("TableData", table_data))

    assert result == ""


def test_export_table_to_csv_missing_df() -> None:
    """Test CSV export when df key is missing."""
    table_data = {"text": "No dataframe key"}
    result = export_table_to_csv(cast("TableData", table_data))

    assert result == ""


def test_export_table_to_tsv(sample_table_data: dict[str, Any]) -> None:
    """Test TSV export functionality."""
    result = export_table_to_tsv(cast("TableData", sample_table_data))

    assert "Name\tAge\tScore\tActive" in result
    assert "Alice\t25\t95.5\tTrue" in result


def test_enhance_table_markdown_basic(sample_table_data: dict[str, Any]) -> None:
    """Test basic markdown enhancement."""
    result = enhance_table_markdown(cast("TableData", sample_table_data))

    lines = result.split("\n")

    assert lines[0] == "| Name | Age | Score | Active |"

    assert lines[1] == "| --- | ---: | ---: | --- |"

    assert lines[2] == "| Alice | 25 | 95.50 | True |"
    assert lines[3] == "| Bob | 30 | 87.00 | False |"


def test_enhance_table_markdown_numeric_alignment(numeric_table_data: dict[str, Any]) -> None:
    """Test markdown enhancement with numeric column alignment."""
    result = enhance_table_markdown(cast("TableData", numeric_table_data))

    lines = result.split("\n")

    assert lines[1] == "| ---: | ---: | ---: |"

    assert lines[2] == "| 1000.50 | 150 | 15.00 |"


def test_enhance_table_markdown_empty_table(empty_table_data: dict[str, Any]) -> None:
    """Test markdown enhancement with empty table."""
    result = enhance_table_markdown(cast("TableData", empty_table_data))

    assert result == "Empty table"


def test_enhance_table_markdown_no_df() -> None:
    """Test markdown enhancement when df is None."""
    table_data = {"text": "No dataframe", "df": None}
    result = enhance_table_markdown(cast("TableData", table_data))

    assert result == "No dataframe"


def test_enhance_table_markdown_missing_text() -> None:
    """Test markdown enhancement when text is missing."""
    table_data = {"df": None}
    result = enhance_table_markdown(cast("TableData", table_data))

    assert result == ""


def test_enhance_table_markdown_with_nulls(table_with_nulls: dict[str, Any]) -> None:
    """Test markdown enhancement with null values."""
    result = enhance_table_markdown(cast("TableData", table_with_nulls))

    lines = result.split("\n")

    assert lines[2] == "| A | 10 | X |"
    assert lines[3] == "| B |  | Y |"
    assert lines[4] == "|  | 30 | Z |"


def test_enhance_table_markdown_with_pipes() -> None:
    """Test markdown enhancement with pipe characters that need escaping."""
    df = pd.DataFrame({"Text": ["Hello|World", "Test|Data"], "Value": [1, 2]})
    table_data = {"df": df}

    result = enhance_table_markdown(cast("TableData", table_data))

    assert "Hello\\|World" in result
    assert "Test\\|Data" in result


def test_generate_table_summary_empty() -> None:
    """Test table summary generation with empty list."""
    result = generate_table_summary([])

    expected = {
        "table_count": 0,
        "total_rows": 0,
        "total_columns": 0,
        "pages_with_tables": 0,
    }

    assert result == expected


def test_generate_table_summary_single_table(sample_table_data: dict[str, Any]) -> None:
    """Test table summary generation with single table."""
    result = generate_table_summary([cast("TableData", sample_table_data)])

    assert result["table_count"] == 1
    assert result["total_rows"] == 3
    assert result["total_columns"] == 4
    assert result["pages_with_tables"] == 1
    assert result["avg_rows_per_table"] == 3.0
    assert result["avg_columns_per_table"] == 4.0
    assert result["tables_by_page"] == {1: 1}


def test_generate_table_summary_multiple_tables(
    sample_table_data: dict[str, Any], numeric_table_data: dict[str, Any], empty_table_data: dict[str, Any]
) -> None:
    """Test table summary generation with multiple tables."""
    tables = [
        cast("TableData", sample_table_data),
        cast("TableData", numeric_table_data),
        cast("TableData", empty_table_data),
    ]
    result = generate_table_summary(tables)

    assert result["table_count"] == 3
    assert result["total_rows"] == 6
    assert result["total_columns"] == 7
    assert result["pages_with_tables"] == 3
    assert result["avg_rows_per_table"] == 2.0
    assert result["avg_columns_per_table"] == pytest.approx(2.33, rel=1e-2)
    assert result["tables_by_page"] == {1: 1, 2: 1, 3: 1}


def test_generate_table_summary_no_df() -> None:
    """Test table summary with tables that have no df."""
    table_data = {"text": "No dataframe", "page_number": 1}
    result = generate_table_summary([cast("TableData", table_data)])

    assert result["table_count"] == 1
    assert result["total_rows"] == 0
    assert result["total_columns"] == 0
    assert result["pages_with_tables"] == 1


def test_generate_table_summary_same_page() -> None:
    """Test table summary with multiple tables on same page."""
    table1 = {"df": pd.DataFrame({"A": [1]}), "page_number": 1}
    table2 = {"df": pd.DataFrame({"B": [2]}), "page_number": 1}

    result = generate_table_summary([cast("TableData", table1), cast("TableData", table2)])

    assert result["table_count"] == 2
    assert result["pages_with_tables"] == 1
    assert result["tables_by_page"] == {1: 2}


def test_extract_table_structure_info_basic(sample_table_data: dict[str, Any]) -> None:
    """Test basic table structure extraction."""
    result = extract_table_structure_info(cast("TableData", sample_table_data))

    assert result["has_headers"] is True
    assert result["row_count"] == 3
    assert result["column_count"] == 4
    assert result["numeric_columns"] == 2
    assert result["text_columns"] == 2
    assert result["empty_cells"] == 0
    assert result["data_density"] == 1.0


def test_extract_table_structure_info_empty(empty_table_data: dict[str, Any]) -> None:
    """Test structure extraction with empty table."""
    result = extract_table_structure_info(cast("TableData", empty_table_data))

    assert result["has_headers"] is False
    assert result["row_count"] == 0
    assert result["column_count"] == 0
    assert result["numeric_columns"] == 0
    assert result["text_columns"] == 0
    assert result["empty_cells"] == 0
    assert result["data_density"] == 0.0


def test_extract_table_structure_info_no_df() -> None:
    """Test structure extraction when df is None."""
    table_data = {"text": "No dataframe", "df": None}
    result = extract_table_structure_info(cast("TableData", table_data))

    expected = {
        "has_headers": False,
        "row_count": 0,
        "column_count": 0,
        "numeric_columns": 0,
        "text_columns": 0,
        "empty_cells": 0,
        "data_density": 0.0,
    }

    assert result == expected


def test_extract_table_structure_info_with_nulls(table_with_nulls: dict[str, Any]) -> None:
    """Test structure extraction with null values."""
    result = extract_table_structure_info(cast("TableData", table_with_nulls))

    assert result["row_count"] == 4
    assert result["column_count"] == 3
    assert result["empty_cells"] == 3
    assert result["data_density"] == 0.75


def test_is_numeric_column_detection(mixed_type_table: dict[str, Any]) -> None:
    """Test numeric column detection with mixed types."""
    from kreuzberg._utils._table import _is_numeric_column

    df = mixed_type_table["df"]

    assert _is_numeric_column(df["ID"])
    assert _is_numeric_column(df["Amount"])
    assert _is_numeric_column(df["Percentage"])
    assert not _is_numeric_column(df["Text"])


def test_is_numeric_column_edge_cases() -> None:
    """Test numeric column detection edge cases."""
    from kreuzberg._utils._table import _is_numeric_column

    empty_series = pd.Series([], dtype=object)
    assert not _is_numeric_column(empty_series)

    null_series = pd.Series([None, None, None])
    assert not _is_numeric_column(null_series)

    int_series = pd.Series([1, 2, 3], dtype="int64")
    assert _is_numeric_column(int_series)

    float_series = pd.Series([1.1, 2.2, 3.3], dtype="float64")
    assert _is_numeric_column(float_series)

    mixed_series = pd.Series(["1", "2", "3", "not_a_number"])
    assert _is_numeric_column(mixed_series)

    mostly_text = pd.Series(["a", "b", "c", "1"])
    assert not _is_numeric_column(mostly_text)


def test_is_numeric_column_large_series() -> None:
    """Test numeric column detection with large series (sampling)."""
    from kreuzberg._utils._table import _is_numeric_column

    large_series = pd.Series([str(i) for i in range(2000)] + ["text"] * 100)
    assert _is_numeric_column(large_series)


def test_is_numeric_column_special_formats() -> None:
    """Test numeric column detection with special number formats."""
    from kreuzberg._utils._table import _is_numeric_column

    currency_series = pd.Series(["$1,234.56", "$2,500.00", "$999.99"])
    assert _is_numeric_column(currency_series)

    percent_series = pd.Series(["15%", "20.5%", "8%"])
    assert _is_numeric_column(percent_series)

    scientific_series = pd.Series(["1.23e10", "4.56E-5", "7.89e+3"])
    assert _is_numeric_column(scientific_series)


def test_is_numeric_column_error_handling() -> None:
    """Test numeric column detection error handling."""
    from kreuzberg._utils._table import _is_numeric_column

    problematic_series = pd.Series([float("inf"), float("-inf"), float("nan"), "1", "2"])

    result = _is_numeric_column(problematic_series)
    assert isinstance(result, bool)


def test_table_formatting_edge_cases() -> None:
    """Test edge cases in table formatting."""
    df = pd.DataFrame({"Whole Numbers": [1.0, 2.0, 3.0], "Decimals": [1.23, 4.56, 7.89]})
    table_data = {"df": df}

    result = enhance_table_markdown(cast("TableData", table_data))

    assert "| 1 | 1.23 |" in result
    assert "| 2 | 4.56 |" in result
    assert "| 3 | 7.89 |" in result
