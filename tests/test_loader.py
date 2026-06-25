"""Synchronous test suite for XbergLoader."""

from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from xberg import ExtractionConfig, OcrConfig, PageConfig

from langchain_xberg import XbergLoader
from tests.conftest import make_mock_keyword, make_mock_result, make_mock_table

# --- Constructor validation ---


def test_no_input_raises() -> None:
    with pytest.raises(ValueError, match="Either 'file_path' or 'data'"):
        XbergLoader()


def test_both_inputs_raises() -> None:
    with pytest.raises(ValueError, match="Cannot specify both"):
        XbergLoader(file_path="test.pdf", data=b"test")


def test_bytes_requires_mime_type() -> None:
    with pytest.raises(ValueError, match="'mime_type' is required"):
        XbergLoader(data=b"test")


def test_valid_file_path() -> None:
    loader = XbergLoader(file_path="test.pdf")
    assert loader._file_path == Path("test.pdf")


def test_valid_bytes_input() -> None:
    loader = XbergLoader(data=b"test", mime_type="text/plain")
    assert loader._data == b"test"
    assert loader._mime_type == "text/plain"


def test_valid_multiple_files() -> None:
    loader = XbergLoader(file_path=["a.pdf", "b.docx"])
    assert loader._file_path == [Path("a.pdf"), Path("b.docx")]


def test_valid_path_object() -> None:
    loader = XbergLoader(file_path=Path("test.pdf"))
    assert loader._file_path == Path("test.pdf")


# --- Config handling ---


def test_default_config() -> None:
    loader = XbergLoader(file_path="test.pdf")
    assert isinstance(loader._config, ExtractionConfig)
    assert loader._config.output_format == "plain"


def test_custom_config_passthrough() -> None:
    custom_config = ExtractionConfig(
        output_format="html",
        force_ocr=True,
        ocr=OcrConfig(backend="paddleocr"),
    )
    loader = XbergLoader(file_path="test.pdf", config=custom_config)
    assert loader._config is custom_config
    assert loader._config.output_format == "html"


# --- Synchronous loading ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_load_single_text_file(mock_extract: MagicMock, sample_txt_path: Path) -> None:
    mock_extract.return_value = make_mock_result(
        content="Sample text content",
        metadata={"format_type": "text", "word_count": 5},
    )

    loader = XbergLoader(file_path=str(sample_txt_path))
    docs = loader.load()

    assert len(docs) == 1
    assert docs[0].page_content == "Sample text content"
    assert docs[0].metadata["source"] == str(sample_txt_path)
    mock_extract.assert_called_once()


@patch("langchain_xberg.loader.extract_file_sync")
def test_load_single_pdf(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        content="PDF content",
        mime_type="application/pdf",
        metadata={
            "format_type": "pdf",
            "pdf_version": "1.7",
            "producer": "Test",
            "page_count": 3,
        },
        page_count=3,
    )

    loader = XbergLoader(file_path="document.pdf")
    docs = loader.load()

    assert len(docs) == 1
    assert docs[0].page_content == "PDF content"
    assert docs[0].metadata["mime_type"] == "application/pdf"
    assert docs[0].metadata["format_type"] == "pdf"
    assert docs[0].metadata["pdf_version"] == "1.7"
    assert docs[0].metadata["page_count"] == 3


@patch("langchain_xberg.loader.extract_bytes_sync")
def test_load_bytes_mode(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(content="Bytes content")

    loader = XbergLoader(data=b"raw data", mime_type="text/plain")
    docs = loader.load()

    assert len(docs) == 1
    assert docs[0].page_content == "Bytes content"
    assert docs[0].metadata["source"] == "bytes://text/plain"
    mock_extract.assert_called_once()


@patch("langchain_xberg.loader.batch_extract_files_sync")
def test_load_multiple_files(mock_batch: MagicMock) -> None:
    mock_batch.return_value = [make_mock_result(), make_mock_result(), make_mock_result()]

    loader = XbergLoader(file_path=["a.txt", "b.txt", "c.txt"])
    docs = loader.load()

    assert len(docs) == 3
    mock_batch.assert_called_once()
    sources = [d.metadata["source"] for d in docs]
    assert sources == ["a.txt", "b.txt", "c.txt"]


@patch("langchain_xberg.loader.batch_extract_files_sync")
def test_load_directory_with_glob(mock_batch: MagicMock, tmp_dir_with_files: Path) -> None:
    mock_batch.return_value = [make_mock_result(), make_mock_result()]

    loader = XbergLoader(file_path=str(tmp_dir_with_files), glob="*.txt")
    docs = loader.load()

    # Only top-level .txt files (file1.txt, file2.txt)
    assert len(docs) == 2
    mock_batch.assert_called_once()


@patch("langchain_xberg.loader.batch_extract_files_sync")
def test_load_directory_default_glob(mock_batch: MagicMock, tmp_dir_with_files: Path) -> None:
    mock_batch.return_value = [make_mock_result(), make_mock_result(), make_mock_result()]

    loader = XbergLoader(file_path=str(tmp_dir_with_files))
    docs = loader.load()

    # Default glob **/* matches all files including subdir/file3.txt
    assert len(docs) == 3
    mock_batch.assert_called_once()


def test_load_empty_directory(tmp_path: Path) -> None:
    loader = XbergLoader(file_path=str(tmp_path))
    docs = loader.load()

    assert len(docs) == 0


# --- Per-page splitting ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_per_page_splitting(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        pages=[
            {"page_number": 1, "content": "Page 1 text", "tables": [], "images": [], "is_blank": False},
            {"page_number": 2, "content": "Page 2 text", "tables": [], "images": [], "is_blank": False},
            {"page_number": 3, "content": "", "tables": [], "images": [], "is_blank": True},
        ],
        page_count=3,
    )

    config = ExtractionConfig(pages=PageConfig(extract_pages=True))
    loader = XbergLoader(file_path="doc.pdf", config=config)
    docs = loader.load()

    assert len(docs) == 3
    assert docs[0].page_content == "Page 1 text"
    assert docs[1].page_content == "Page 2 text"
    assert docs[2].page_content == ""


@patch("langchain_xberg.loader.extract_file_sync")
def test_per_page_metadata(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        pages=[
            {"page_number": 1, "content": "Page 1", "tables": [], "images": [], "is_blank": False},
            {"page_number": 2, "content": "Page 2", "tables": [], "images": [], "is_blank": True},
        ],
        page_count=2,
    )

    config = ExtractionConfig(pages=PageConfig(extract_pages=True))
    loader = XbergLoader(file_path="doc.pdf", config=config)
    docs = loader.load()

    # Page numbers are 0-indexed in LangChain convention
    assert docs[0].metadata["page"] == 0
    assert docs[0].metadata["is_blank"] is False
    assert docs[1].metadata["page"] == 1
    assert docs[1].metadata["is_blank"] is True


@patch("langchain_xberg.loader.extract_file_sync")
def test_per_page_with_tables(mock_extract: MagicMock) -> None:
    page_table = {"markdown": "| X |\n|---|\n| Y |"}
    mock_extract.return_value = make_mock_result(
        pages=[
            {
                "page_number": 1,
                "content": "Text",
                "tables": [page_table],
                "images": [],
                "is_blank": False,
            },
        ],
        page_count=1,
    )

    config = ExtractionConfig(pages=PageConfig(extract_pages=True))
    loader = XbergLoader(file_path="doc.pdf", config=config)
    docs = loader.load()

    assert "| X |" in docs[0].page_content


@patch("langchain_xberg.loader.extract_file_sync")
def test_per_page_fallback_when_no_pages(mock_extract: MagicMock) -> None:
    """When per_page is configured but result has no pages, fall back to whole document."""
    mock_extract.return_value = make_mock_result(content="Whole document", pages=None)

    config = ExtractionConfig(pages=PageConfig(extract_pages=True))
    loader = XbergLoader(file_path="doc.txt", config=config)
    docs = loader.load()

    assert len(docs) == 1
    assert docs[0].page_content == "Whole document"


# --- Metadata extraction ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_metadata_source_key(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result()

    loader = XbergLoader(file_path="doc.txt")
    docs = loader.load()

    assert "source" in docs[0].metadata
    assert docs[0].metadata["source"] == "doc.txt"


@patch("langchain_xberg.loader.extract_file_sync")
def test_metadata_flattening(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        metadata={
            "format_type": "text",
            "title": "Test Doc",
            "authors": ["Alice", "Bob"],
            "keywords": None,  # Should be dropped
        },
    )

    loader = XbergLoader(file_path="doc.txt")
    docs = loader.load()

    meta = docs[0].metadata
    assert meta["format_type"] == "text"
    assert meta["title"] == "Test Doc"
    assert meta["authors"] == ["Alice", "Bob"]
    assert "keywords" not in meta  # None values dropped


@patch("langchain_xberg.loader.extract_file_sync")
def test_metadata_enrichment(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        quality_score=0.85,
        detected_languages=["eng", "deu"],
        output_format="markdown",
    )

    loader = XbergLoader(file_path="doc.txt")
    docs = loader.load()

    meta = docs[0].metadata
    assert meta["quality_score"] == 0.85
    assert meta["detected_languages"] == ["eng", "deu"]
    assert meta["output_format"] == "markdown"
    assert meta["mime_type"] == "text/plain"
    assert meta["page_count"] == 1


@patch("langchain_xberg.loader.extract_file_sync")
def test_extracted_keywords_in_metadata(mock_extract: MagicMock) -> None:
    kw1 = make_mock_keyword(text="python", score=0.95, algorithm="yake")
    kw2 = make_mock_keyword(text="machine learning", score=0.88, algorithm="yake")
    mock_extract.return_value = make_mock_result(extracted_keywords=[kw1, kw2])

    loader = XbergLoader(file_path="doc.txt")
    docs = loader.load()

    keywords = docs[0].metadata["extracted_keywords"]
    assert len(keywords) == 2
    assert keywords[0] == {"text": "python", "score": 0.95, "algorithm": "yake"}
    assert keywords[1]["text"] == "machine learning"


@patch("langchain_xberg.loader.extract_file_sync")
def test_processing_warnings_in_metadata(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result(
        processing_warnings=["Low quality scan detected", "Missing font fallback"]
    )

    loader = XbergLoader(file_path="doc.txt")
    docs = loader.load()

    assert "processing_warnings" in docs[0].metadata
    warnings = docs[0].metadata["processing_warnings"]
    assert len(warnings) == 2
    assert warnings[0] == {"source": "extraction", "message": "Low quality scan detected"}
    assert warnings[1] == {"source": "extraction", "message": "Missing font fallback"}


# --- Table extraction ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_table_extraction_in_content(mock_extract: MagicMock) -> None:
    table = make_mock_table(markdown="| Col1 | Col2 |\n|---|---|\n| A | B |")
    mock_extract.return_value = make_mock_result(content="Main text", tables=[table])

    loader = XbergLoader(file_path="doc.pdf")
    docs = loader.load()

    assert "Main text" in docs[0].page_content
    assert "| Col1 | Col2 |" in docs[0].page_content


@patch("langchain_xberg.loader.extract_file_sync")
def test_table_extraction_in_metadata(mock_extract: MagicMock) -> None:
    table = make_mock_table(
        cells=[["A", "B"], ["1", "2"]],
        markdown="| A | B |\n|---|---|\n| 1 | 2 |",
        page_number=1,
    )
    mock_extract.return_value = make_mock_result(tables=[table])

    loader = XbergLoader(file_path="doc.pdf")
    docs = loader.load()

    meta = docs[0].metadata
    assert meta["table_count"] == 1
    assert len(meta["tables"]) == 1
    assert meta["tables"][0]["cells"] == [["A", "B"], ["1", "2"]]
    assert meta["tables"][0]["page_number"] == 1


@patch("langchain_xberg.loader.extract_file_sync")
def test_multiple_tables_in_content(mock_extract: MagicMock) -> None:
    t1 = make_mock_table(markdown="| T1 |")
    t2 = make_mock_table(markdown="| T2 |")
    mock_extract.return_value = make_mock_result(content="Text", tables=[t1, t2])

    loader = XbergLoader(file_path="doc.pdf")
    docs = loader.load()

    # Tables separated by double newlines
    assert docs[0].page_content == "Text\n\n| T1 |\n\n| T2 |"


# --- Error propagation ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_error_propagation(mock_extract: MagicMock) -> None:
    from xberg.exceptions import XbergError

    mock_extract.side_effect = XbergError("Extraction failed")

    loader = XbergLoader(file_path="bad.pdf")

    with pytest.raises(XbergError, match=r"Failed to extract 'bad\.pdf'"):
        loader.load()


@patch("langchain_xberg.loader.batch_extract_files_sync")
def test_batch_error_propagation(mock_batch: MagicMock) -> None:
    from xberg.exceptions import XbergError

    error_result = make_mock_result(
        content="Error: unsupported format",
        mime_type="text/plain",
        metadata={"error": {"error_type": "ParsingError", "message": "unsupported format"}},
    )
    mock_batch.return_value = [make_mock_result(), error_result]

    loader = XbergLoader(file_path=["good.txt", "bad.xyz"])

    with pytest.raises(XbergError, match=r"Failed to extract 'bad\.xyz'"):
        loader.load()


# --- Lazy loading ---


@patch("langchain_xberg.loader.extract_file_sync")
def test_lazy_load_is_iterator(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_mock_result()

    loader = XbergLoader(file_path="doc.txt")
    result = loader.lazy_load()

    # Should be an iterator, not a list
    assert hasattr(result, "__next__")


@patch("langchain_xberg.loader.batch_extract_files_sync")
def test_lazy_load_yields_documents(mock_batch: MagicMock) -> None:
    mock_batch.return_value = [make_mock_result(), make_mock_result()]

    loader = XbergLoader(file_path=["a.txt", "b.txt"])

    docs = []
    for doc in loader.lazy_load():
        docs.append(doc)

    assert len(docs) == 2
