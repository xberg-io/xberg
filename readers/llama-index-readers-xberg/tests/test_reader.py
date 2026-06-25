"""Tests for XbergReader."""

import base64
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from xberg import ExtractionConfig, OcrConfig, PageConfig
from llama_index.core.readers.base import BasePydanticReader
from llama_index.readers.xberg import XbergReader
from llama_index.readers.xberg._utils import build_metadata, generate_doc_id
from pydantic import ValidationError

from tests.conftest import make_extraction_result, make_page_content


def test_class() -> None:
    names_of_base_classes = [b.__name__ for b in XbergReader.__mro__]
    assert BasePydanticReader.__name__ in names_of_base_classes


def test_class_name() -> None:
    reader = XbergReader()
    assert reader.class_name() == "XbergReader"


def test_is_remote_false() -> None:
    reader = XbergReader()
    assert reader.is_remote is False


def test_default_fields() -> None:
    reader = XbergReader()
    assert reader.raise_on_error is False
    assert reader.extraction_config is None


def test_to_dict_without_config() -> None:
    reader = XbergReader()
    d = reader.to_dict()
    assert d["extraction_config"] is None
    assert d["raise_on_error"] is False


def test_to_dict_with_config() -> None:
    config = ExtractionConfig(output_format="markdown", force_ocr=True)
    reader = XbergReader(extraction_config=config)
    d = reader.to_dict()
    assert isinstance(d["extraction_config"], dict)
    assert d["extraction_config"]["output_format"] == "markdown"
    assert d["extraction_config"]["force_ocr"] is True


def test_from_dict_round_trip() -> None:
    config = ExtractionConfig(
        output_format="markdown",
        ocr=OcrConfig(backend="paddleocr", language="fra"),
        pages=PageConfig(extract_pages=True),
    )
    reader = XbergReader(extraction_config=config, raise_on_error=True)
    d = reader.to_dict()
    restored = XbergReader.from_dict(d)
    assert restored.raise_on_error is True
    assert isinstance(restored.extraction_config, ExtractionConfig)
    assert restored.extraction_config.output_format == "markdown"
    assert restored.extraction_config.ocr.backend == "paddleocr"
    assert restored.extraction_config.pages.extract_pages is True


def test_accepts_dict_as_extraction_config() -> None:
    reader = XbergReader(extraction_config={"output_format": "markdown", "force_ocr": True})
    assert isinstance(reader.extraction_config, ExtractionConfig)
    assert reader.extraction_config.output_format == "markdown"


def test_rejects_invalid_extraction_config() -> None:
    with pytest.raises(ValidationError, match="Expected ExtractionConfig"):
        XbergReader(extraction_config=42)


def test_standard_metadata_fields() -> None:

    result = make_extraction_result(page_count=5)
    meta = build_metadata(result=result, file_path=Path("/tmp/test.pdf"))
    assert meta["file_name"] == "test.pdf"
    assert meta["file_path"] == "/tmp/test.pdf"
    assert meta["file_type"] == "application/pdf"
    assert meta["total_pages"] == 5


def test_document_metadata_fields() -> None:

    result = make_extraction_result()
    meta = build_metadata(result=result, file_path=Path("/tmp/test.pdf"))
    assert meta["title"] == "Test Document"
    assert meta["authors"] == ["Author One"]
    assert meta["language"] == "eng"
    assert meta["format_type"] == "pdf"


def test_extraction_result_fields() -> None:

    result = make_extraction_result(quality_score=0.88)
    meta = build_metadata(result=result, file_path=Path("/tmp/test.pdf"))
    assert meta["quality_score"] == 0.88
    assert meta["detected_languages"] == ["eng"]


def test_extra_info_overrides() -> None:

    result = make_extraction_result()
    meta = build_metadata(
        result=result,
        file_path=Path("/tmp/test.pdf"),
        extra_info={"title": "Override", "custom": "value"},
    )
    assert meta["title"] == "Override"
    assert meta["custom"] == "value"


def test_bytes_source_metadata() -> None:

    result = make_extraction_result()
    meta = build_metadata(result=result, source="bytes_input")
    assert meta["file_name"] == "bytes_input"
    assert meta["file_path"] == "bytes_input"


def test_file_path_id_deterministic() -> None:

    path = Path("/tmp/test.pdf")
    assert generate_doc_id(file_path=path) == generate_doc_id(file_path=path)


def test_file_path_id_with_page() -> None:

    path = Path("/tmp/test.pdf")
    id_no_page = generate_doc_id(file_path=path)
    id_page_1 = generate_doc_id(file_path=path, page_number=1)
    id_page_2 = generate_doc_id(file_path=path, page_number=2)
    assert id_no_page != id_page_1
    assert id_page_1 != id_page_2


def test_bytes_id_deterministic() -> None:

    assert generate_doc_id(data=b"hello") == generate_doc_id(data=b"hello")


def test_bytes_id_with_page() -> None:

    assert generate_doc_id(data=b"hello") != generate_doc_id(data=b"hello", page_number=1)


def test_different_paths_different_ids() -> None:

    assert generate_doc_id(file_path=Path("/tmp/a.pdf")) != generate_doc_id(file_path=Path("/tmp/b.pdf"))


def test_generate_doc_id_no_input_raises() -> None:
    with pytest.raises(ValueError, match="Either file_path or data must be provided"):
        generate_doc_id()


def test_prepare_single_file() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(file_path=Path("/tmp/test.pdf"))
    assert task.kind == "file"
    assert task.paths == (Path("/tmp/test.pdf"),)


def test_prepare_single_file_from_string() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(file_path="/tmp/test.pdf")
    assert task.kind == "file"
    assert task.paths == (Path("/tmp/test.pdf"),)


def test_prepare_single_element_list_routes_to_single() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(file_path=[Path("/tmp/test.pdf")])
    assert task.kind == "file"
    assert task.paths == (Path("/tmp/test.pdf"),)


def test_prepare_batch_files() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(file_path=[Path("/tmp/a.pdf"), Path("/tmp/b.pdf")])
    assert task.kind == "file_batch"
    assert task.paths == (Path("/tmp/a.pdf"), Path("/tmp/b.pdf"))


def test_prepare_single_bytes() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(data=b"pdf", mime_type="application/pdf")
    assert task.kind == "bytes"
    assert task.data_list == (b"pdf",)
    assert task.mime_types == ("application/pdf",)


def test_prepare_batch_bytes() -> None:
    reader = XbergReader()
    task = reader._prepare_extractions(data=[b"a", b"b"], mime_type=["application/pdf", "text/plain"])
    assert task.kind == "bytes_batch"
    assert task.data_list == (b"a", b"b")
    assert task.mime_types == ("application/pdf", "text/plain")


def test_prepare_no_input_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="Either file_path or data"):
        reader._prepare_extractions()


def test_prepare_bytes_without_mime_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="mime_type must be a string"):
        reader._prepare_extractions(data=b"pdf")


def test_prepare_bytes_with_list_mime_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="mime_type must be a string"):
        reader._prepare_extractions(data=b"pdf", mime_type=["application/pdf"])


def test_prepare_batch_bytes_length_mismatch_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="parallel lists of equal length"):
        reader._prepare_extractions(data=[b"a", b"b"], mime_type=["application/pdf"])


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_single_file_returns_document(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result(content="Hello PDF")
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert len(docs) == 1
    assert docs[0].text == "Hello PDF"
    mock_extract.assert_called_once()


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_single_file_metadata(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result()
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    meta = docs[0].metadata
    assert meta["file_name"] == "test.pdf"
    assert meta["file_type"] == "application/pdf"
    assert meta["title"] == "Test Document"


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_single_file_deterministic_id(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result()
    reader = XbergReader()
    docs1 = reader.load_data(Path("/tmp/test.pdf"))
    docs2 = reader.load_data(Path("/tmp/test.pdf"))
    assert docs1[0].id_ == docs2[0].id_


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_extra_info_merged(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result()
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"), extra_info={"custom": "value"})
    assert docs[0].metadata["custom"] == "value"


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_string_path_accepted(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result()
    reader = XbergReader()
    docs = reader.load_data("/tmp/test.pdf")
    assert len(docs) == 1


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_extraction_config_passed(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result()
    config = ExtractionConfig(output_format="markdown")
    reader = XbergReader(extraction_config=config)
    reader.load_data(Path("/tmp/test.pdf"))
    assert mock_extract.call_args is not None


@patch("llama_index.readers.xberg.base.batch_extract_files_sync")
def test_batch_files(mock_batch: MagicMock) -> None:
    mock_batch.return_value = [
        make_extraction_result(content="Doc A"),
        make_extraction_result(content="Doc B"),
    ]
    reader = XbergReader()
    docs = reader.load_data([Path("/tmp/a.pdf"), Path("/tmp/b.pdf")])
    assert len(docs) == 2
    assert docs[0].text == "Doc A"
    assert docs[1].text == "Doc B"


@patch("llama_index.readers.xberg.base.batch_extract_files_sync")
def test_batch_unique_ids(mock_batch: MagicMock) -> None:
    mock_batch.return_value = [make_extraction_result(), make_extraction_result()]
    reader = XbergReader()
    docs = reader.load_data([Path("/tmp/a.pdf"), Path("/tmp/b.pdf")])
    assert docs[0].id_ != docs[1].id_


@patch("llama_index.readers.xberg.base.extract_bytes_sync")
def test_single_bytes(mock_extract: MagicMock) -> None:
    mock_extract.return_value = make_extraction_result(content="Bytes content")
    reader = XbergReader()
    docs = reader.load_data(data=b"pdf bytes", mime_type="application/pdf")
    assert len(docs) == 1
    assert docs[0].text == "Bytes content"


@patch("llama_index.readers.xberg.base.batch_extract_bytes_sync")
def test_batch_bytes(mock_batch: MagicMock) -> None:
    mock_batch.return_value = [make_extraction_result(content="A"), make_extraction_result(content="B")]
    reader = XbergReader()
    docs = reader.load_data(data=[b"bytes1", b"bytes2"], mime_type=["application/pdf", "application/pdf"])
    assert len(docs) == 2


def test_bytes_without_mime_type_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="mime_type must be a string"):
        reader.load_data(data=b"bytes")


def test_batch_bytes_length_mismatch_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="parallel lists of equal length"):
        reader.load_data(data=[b"a", b"b"], mime_type=["application/pdf"])


def test_no_input_raises() -> None:
    reader = XbergReader()
    with pytest.raises(ValueError, match="Either file_path or data"):
        reader.load_data()


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_per_page_yields_multiple_documents(mock_extract: MagicMock) -> None:
    result = make_extraction_result(page_count=3)
    result.pages = [
        make_page_content(page_number=1, content="Page 1"),
        make_page_content(page_number=2, content="Page 2"),
        make_page_content(page_number=3, content="Page 3"),
    ]
    mock_extract.return_value = result
    reader = XbergReader(extraction_config=ExtractionConfig(pages=PageConfig(extract_pages=True)))
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert len(docs) == 3
    assert docs[0].text == "Page 1"
    assert docs[1].text == "Page 2"
    assert docs[2].text == "Page 3"


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_per_page_metadata_has_page_number(mock_extract: MagicMock) -> None:
    result = make_extraction_result(page_count=2)
    result.pages = [make_page_content(page_number=1, content="P1"), make_page_content(page_number=2, content="P2")]
    mock_extract.return_value = result
    reader = XbergReader(extraction_config=ExtractionConfig(pages=PageConfig(extract_pages=True)))
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert docs[0].metadata["page_number"] == 1
    assert docs[1].metadata["page_number"] == 2


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_per_page_unique_ids(mock_extract: MagicMock) -> None:
    result = make_extraction_result(page_count=2)
    result.pages = [make_page_content(page_number=1, content="P1"), make_page_content(page_number=2, content="P2")]
    mock_extract.return_value = result
    reader = XbergReader(extraction_config=ExtractionConfig(pages=PageConfig(extract_pages=True)))
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert docs[0].id_ != docs[1].id_


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_tolerant_mode_skips_errors(mock_extract: MagicMock) -> None:
    mock_extract.side_effect = RuntimeError("extraction failed")
    reader = XbergReader(raise_on_error=False)
    docs = reader.load_data(Path("/tmp/bad.pdf"))
    assert len(docs) == 0


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_strict_mode_raises(mock_extract: MagicMock) -> None:
    mock_extract.side_effect = RuntimeError("extraction failed")
    reader = XbergReader(raise_on_error=True)
    with pytest.raises(RuntimeError, match="extraction failed"):
        reader.load_data(Path("/tmp/bad.pdf"))


@patch("llama_index.readers.xberg.base.batch_extract_files_sync")
def test_tolerant_batch_skips_errors(mock_batch: MagicMock) -> None:
    mock_batch.side_effect = RuntimeError("batch failed")
    reader = XbergReader(raise_on_error=False)
    docs = reader.load_data([Path("/tmp/a.pdf"), Path("/tmp/b.pdf")])
    assert len(docs) == 0


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_elements_stored_in_metadata(mock_extract: MagicMock) -> None:
    result = make_extraction_result()
    result.elements = [{"element_type": "title", "text": "Hello"}]
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert "_xberg_elements" in docs[0].metadata
    assert docs[0].metadata["_xberg_elements"] == result.elements


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_elements_excluded_from_llm_keys(mock_extract: MagicMock) -> None:
    result = make_extraction_result()
    result.elements = [{"element_type": "title", "text": "Hello"}]
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert "_xberg_elements" in docs[0].excluded_llm_metadata_keys
    assert "_xberg_elements" in docs[0].excluded_embed_metadata_keys


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_no_elements_when_unified(mock_extract: MagicMock) -> None:
    result = make_extraction_result()
    result.elements = None
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert "_xberg_elements" not in docs[0].metadata
    assert docs[0].excluded_llm_metadata_keys == []


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_images_base64_encoded(mock_extract: MagicMock) -> None:
    mock_img = {
        "data": b"\x89PNG\r\n",
        "format": "PNG",
        "image_index": 0,
        "page_number": 1,
        "width": 100,
        "height": 200,
        "colorspace": "RGB",
        "bits_per_component": 8,
        "is_mask": False,
        "description": "test image",
    }
    result = make_extraction_result()
    result.images = [mock_img]
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    images = docs[0].metadata["images"]
    assert len(images) == 1
    assert images[0]["data"] == base64.b64encode(b"\x89PNG\r\n").decode("ascii")
    assert images[0]["format"] == "PNG"
    assert images[0]["width"] == 100


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_table_appended_when_not_inlined(mock_extract: MagicMock) -> None:
    mock_table = MagicMock()
    mock_table.markdown = "| A | B |\n|---|---|\n| 1 | 2 |"
    result = make_extraction_result(content="Main text")
    result.tables = [mock_table]
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert "| A | B |" in docs[0].text
    assert "Main text" in docs[0].text


@patch("llama_index.readers.xberg.base.extract_file_sync")
def test_table_not_duplicated_when_inlined(mock_extract: MagicMock) -> None:
    table_md = "| A | B |\n|---|---|\n| 1 | 2 |"
    mock_table = MagicMock()
    mock_table.markdown = table_md
    result = make_extraction_result(content=f"Text before\n\n{table_md}\n\nText after")
    result.tables = [mock_table]
    mock_extract.return_value = result
    reader = XbergReader()
    docs = reader.load_data(Path("/tmp/test.pdf"))
    assert docs[0].text.count("| A | B |") == 1


@patch("llama_index.readers.xberg.base.extract_file", new_callable=AsyncMock)
async def test_aload_data_single_file(mock_extract: AsyncMock) -> None:
    mock_extract.return_value = make_extraction_result(content="Async content")
    reader = XbergReader()
    docs = await reader.aload_data(Path("/tmp/test.pdf"))
    assert len(docs) == 1
    assert docs[0].text == "Async content"


@patch("llama_index.readers.xberg.base.batch_extract_files", new_callable=AsyncMock)
async def test_aload_data_batch(mock_batch: AsyncMock) -> None:
    mock_batch.return_value = [make_extraction_result(content="A"), make_extraction_result(content="B")]
    reader = XbergReader()
    docs = await reader.aload_data([Path("/tmp/a.pdf"), Path("/tmp/b.pdf")])
    assert len(docs) == 2


@patch("llama_index.readers.xberg.base.extract_bytes", new_callable=AsyncMock)
async def test_aload_data_bytes(mock_extract: AsyncMock) -> None:
    mock_extract.return_value = make_extraction_result(content="Async bytes")
    reader = XbergReader()
    docs = await reader.aload_data(data=b"pdf", mime_type="application/pdf")
    assert len(docs) == 1


@patch("llama_index.readers.xberg.base.extract_file", new_callable=AsyncMock)
async def test_async_error_tolerant(mock_extract: AsyncMock) -> None:
    mock_extract.side_effect = RuntimeError("async fail")
    reader = XbergReader(raise_on_error=False)
    docs = await reader.aload_data(Path("/tmp/bad.pdf"))
    assert len(docs) == 0


@patch("llama_index.readers.xberg.base.extract_file", new_callable=AsyncMock)
async def test_async_error_strict(mock_extract: AsyncMock) -> None:
    mock_extract.side_effect = RuntimeError("async fail")
    reader = XbergReader(raise_on_error=True)
    with pytest.raises(RuntimeError, match="async fail"):
        await reader.aload_data(Path("/tmp/bad.pdf"))
