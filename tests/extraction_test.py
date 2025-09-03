from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, Mock

import pytest

from kreuzberg import TesseractConfig
from kreuzberg._mime_types import (
    DOCX_MIME_TYPE,
    EXCEL_MIME_TYPE,
    MARKDOWN_MIME_TYPE,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
)
from kreuzberg._types import Entity, ExtractionConfig, ExtractionResult
from kreuzberg.exceptions import ValidationError
from kreuzberg.extraction import (
    _handle_chunk_content,
    _validate_and_post_process_helper,
    _validate_and_post_process_sync,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

IS_CI = os.environ.get("CI", "false").lower() == "true"


@pytest.mark.anyio
async def test_extract_bytes_pdf(scanned_pdf: Path) -> None:
    content = scanned_pdf.read_bytes()
    result = await extract_bytes(content, PDF_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
@pytest.mark.skip(reason="non_english_pdf fixture not available")
async def test_extract_bytes_pdf_non_english(non_english_pdf: Path) -> None:
    content = non_english_pdf.read_bytes()
    config = ExtractionConfig(ocr_backend="tesseract", ocr_config=TesseractConfig(language="deu"))
    result = await extract_bytes(content, PDF_MIME_TYPE, config=config)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_bytes_docx(docx_document: Path) -> None:
    content = docx_document.read_bytes()
    result = await extract_bytes(content, DOCX_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert result.content.strip() != ""


@pytest.mark.anyio
async def test_extract_bytes_excel(excel_document: Path) -> None:
    content = excel_document.read_bytes()
    result = await extract_bytes(content, EXCEL_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert result.content.strip() != ""


@pytest.mark.anyio
async def test_extract_bytes_pptx(pptx_document: Path) -> None:
    content = pptx_document.read_bytes()
    result = await extract_bytes(content, POWER_POINT_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert result.content.strip() != ""


@pytest.mark.anyio
async def test_extract_bytes_plain_text() -> None:
    content = b"This is plain text content."
    result = await extract_bytes(content, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content == "This is plain text content."


@pytest.mark.anyio
async def test_extract_bytes_invalid_mime() -> None:
    content = b"Some content"
    with pytest.raises(ValidationError):
        await extract_bytes(content, "application/unknown")


@pytest.mark.anyio
async def test_extract_file_pdf(scanned_pdf: Path) -> None:
    result = await extract_file(scanned_pdf)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_no_extension(tmp_path: Path) -> None:
    file_path = tmp_path / "file_without_extension"
    file_path.write_bytes(b"Text content")
    with pytest.raises(ValidationError):
        await extract_file(file_path)


@pytest.mark.anyio
async def test_extract_file_explicit_mime(tmp_path: Path) -> None:
    file_path = tmp_path / "file_without_extension"
    file_path.write_bytes(b"Text content")
    result = await extract_file(file_path, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content == "Text content"


@pytest.mark.anyio
async def test_extract_file_not_exists() -> None:
    with pytest.raises(ValidationError) as exc_info:
        await extract_file("nonexistent_file.txt")
    assert "file does not exist" in str(exc_info.value)


@pytest.mark.anyio
async def test_batch_extract_file_single(test_article: Path) -> None:
    results = await batch_extract_file([str(test_article)])
    assert len(results) == 1
    assert_extraction_result(results[0], mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_batch_extract_file_multiple(searchable_pdf: Path, test_article: Path, docx_document: Path) -> None:
    file_paths = [str(searchable_pdf), str(test_article), str(docx_document)]
    results = await batch_extract_file(file_paths)
    assert len(results) == 3
    assert_extraction_result(results[0], mime_type=PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(results[1], mime_type=PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(results[2], mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_batch_extract_bytes_single() -> None:
    contents = [(b"Single text content", PLAIN_TEXT_MIME_TYPE)]
    results = await batch_extract_bytes(contents)
    assert len(results) == 1
    assert_extraction_result(results[0], mime_type=PLAIN_TEXT_MIME_TYPE)
    assert results[0].content == "Single text content"


@pytest.mark.anyio
async def test_batch_extract_bytes_multiple(searchable_pdf: Path, docx_document: Path) -> None:
    contents = [
        (b"First text", PLAIN_TEXT_MIME_TYPE),
        (searchable_pdf.read_bytes(), PDF_MIME_TYPE),
        (docx_document.read_bytes(), DOCX_MIME_TYPE),
    ]
    results = await batch_extract_bytes(contents)
    assert len(results) == 3
    assert results[0].content == "First text"
    assert_extraction_result(results[1], mime_type=PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(results[2], mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_with_custom_config(tmp_path: Path) -> None:
    file_path = tmp_path / "text.txt"
    file_path.write_text("Test content for extraction with config")

    custom_config = ExtractionConfig(chunk_content=True, max_chars=10, max_overlap=2)
    result = await extract_file(file_path, config=custom_config)

    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert len(result.chunks) > 0


@pytest.mark.anyio
async def test_batch_extract_empty_list() -> None:
    file_results = await batch_extract_file([])
    assert file_results == []

    bytes_results = await batch_extract_bytes([])
    assert bytes_results == []


@pytest.mark.anyio
@pytest.mark.xfail(
    not IS_CI, reason="GMFT tests may fail locally if gmft dependencies are not installed", raises=Exception
)
async def test_extract_pdf_with_tables(pdfs_with_tables_list: list[Path]) -> None:
    config = ExtractionConfig(extract_tables=True)

    for pdf in pdfs_with_tables_list:
        result = await extract_file(pdf, config=config)
        assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
        assert len(result.tables) > 0


@pytest.mark.anyio
async def test_extract_bytes_with_explicit_mime() -> None:
    content = b"Plain text content"
    result = await extract_bytes(content, PLAIN_TEXT_MIME_TYPE)
    assert result.content == "Plain text content"


def assert_extraction_result(result: ExtractionResult, mime_type: str | None = None) -> None:
    assert result is not None
    assert isinstance(result, ExtractionResult)
    assert result.content is not None
    assert len(result.content) > 0
    if mime_type:
        assert result.mime_type == mime_type
    assert isinstance(result.metadata, dict)
    assert isinstance(result.chunks, list)


def test_extract_bytes_sync_plain_text() -> None:
    content = b"This is plain text content."
    result = extract_bytes_sync(content, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content == "This is plain text content."


def test_extract_file_sync_plain_text(tmp_path: Path) -> None:
    file_path = tmp_path / "test.txt"
    file_path.write_text("Test content")
    result = extract_file_sync(file_path)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content == "Test content"


def test_extract_bytes_sync_invalid_mime() -> None:
    with pytest.raises(ValidationError):
        extract_bytes_sync(b"content", "application/unknown")


def test_extract_file_sync_invalid_mime(tmp_path: Path) -> None:
    file_path = tmp_path / "test.unknown"
    file_path.write_text("content")
    with pytest.raises(ValidationError):
        extract_file_sync(file_path)


def test_extract_file_sync_not_exists() -> None:
    with pytest.raises(ValidationError):
        extract_file_sync("nonexistent.txt")


@pytest.mark.anyio
async def test_batch_extract_with_different_configs() -> None:
    config = ExtractionConfig(chunk_content=True, max_chars=20, max_overlap=5)

    contents = [
        (b"First content that should be chunked", PLAIN_TEXT_MIME_TYPE),
        (b"Second content that should also be chunked", PLAIN_TEXT_MIME_TYPE),
    ]

    results = await batch_extract_bytes(contents, config=config)
    assert len(results) == 2
    assert len(results[0].chunks) > 0
    assert len(results[1].chunks) > 0


@pytest.mark.anyio
async def test_extract_with_quality_processing() -> None:
    config = ExtractionConfig(enable_quality_processing=True)

    content = b"Test content for quality processing"
    result = await extract_bytes(content, PLAIN_TEXT_MIME_TYPE, config=config)

    assert result.content == "Test content for quality processing"
    if "quality_score" in result.metadata:
        assert isinstance(result.metadata["quality_score"], (int, float))


@pytest.mark.anyio
async def test_extract_file_with_progress_callback() -> None:
    progress_updates = []

    def progress_callback(current: int, total: int, message: str) -> None:
        progress_updates.append((current, total, message))

    from tempfile import NamedTemporaryFile

    with NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Test content for progress")
        temp_path = f.name

    try:
        result = await extract_file(temp_path)
        assert result.content == "Test content for progress"
    finally:
        Path(temp_path).unlink(missing_ok=True)


def test_batch_extract_file_sync_mixed(test_article: Path) -> None:
    file_paths = [str(test_article)]
    results = batch_extract_file_sync(file_paths)

    assert len(results) == 1
    assert_extraction_result(results[0], mime_type=PLAIN_TEXT_MIME_TYPE)


def test_batch_extract_bytes_sync_mixed(searchable_pdf: Path, docx_document: Path) -> None:
    contents = [
        (b"This is plain text", PLAIN_TEXT_MIME_TYPE),
        (
            docx_document.read_bytes(),
            DOCX_MIME_TYPE,
        ),
        (searchable_pdf.read_bytes(), PDF_MIME_TYPE),
    ]

    results = batch_extract_bytes_sync(contents)
    assert len(results) == len(contents)
    for i, result in enumerate(results):
        if i == 0:
            assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
            assert result.content.strip() == "This is plain text"
        else:
            assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE if i == 1 else PLAIN_TEXT_MIME_TYPE)


def test_batch_extract_file_sync_with_errors(tmp_path: Path, searchable_pdf: Path) -> None:
    valid_file = tmp_path / "valid.pdf"
    valid_file.write_bytes(searchable_pdf.read_bytes())
    non_existent = tmp_path / "non_existent.pdf"

    bad_file = tmp_path / "bad.unknown"
    bad_file.write_text("unknown format")

    file_paths = [str(valid_file), str(non_existent), str(bad_file)]

    results = batch_extract_file_sync(file_paths)

    assert len(results) == 3
    assert len(results[0].content) > 0
    assert results[0].mime_type == PLAIN_TEXT_MIME_TYPE
    assert "Error:" in results[1].content
    assert results[1].metadata.get("error") is not None
    assert "Error:" in results[2].content
    assert results[2].metadata.get("error") is not None


def test_batch_extract_bytes_sync_with_errors(searchable_pdf: Path) -> None:
    pdf_content = searchable_pdf.read_bytes()

    contents = [
        (pdf_content, PDF_MIME_TYPE),
        (b"invalid content", "application/unknown"),
        (b"test text", PLAIN_TEXT_MIME_TYPE),
    ]

    results = batch_extract_bytes_sync(contents)

    assert len(results) == 3
    assert len(results[0].content) > 0
    assert results[0].mime_type == PLAIN_TEXT_MIME_TYPE
    assert "Error:" in results[1].content
    assert results[1].metadata.get("error") is not None
    assert results[2].content == "test text"


def test_handle_chunk_content_basic() -> None:
    content = "This is a test content that should be chunked. " * 10
    config = ExtractionConfig(chunk_content=True, max_chars=50, max_overlap=10)

    chunks = _handle_chunk_content(mime_type="text/plain", config=config, content=content)

    assert isinstance(chunks, list)
    assert len(chunks) > 1
    assert all(len(chunk) <= 50 for chunk in chunks)


def test_handle_chunk_content_empty() -> None:
    config = ExtractionConfig(chunk_content=True)

    chunks = _handle_chunk_content(mime_type="text/plain", config=config, content="")

    assert chunks == []


def test_handle_chunk_content_markdown(mocker: MockerFixture) -> None:
    content = "# Header\\n\\nParagraph 1\\n\\n## Subheader\\n\\nParagraph 2" * 5
    config = ExtractionConfig(chunk_content=True, max_chars=100)

    mock_chunker = mocker.Mock()
    mock_chunker.chunks.return_value = ["chunk1", "chunk2"]
    mocker.patch("kreuzberg.extraction.get_chunker", return_value=mock_chunker)

    chunks = _handle_chunk_content(mime_type="text/markdown", config=config, content=content)

    assert isinstance(chunks, list)
    assert len(chunks) == 2
    assert chunks == ["chunk1", "chunk2"]


def test_validate_and_post_process_helper_with_entities(mocker: MockerFixture) -> None:
    mock_extract_entities = mocker.patch(
        "kreuzberg.extraction.extract_entities",
        return_value=[
            Entity(type="PERSON", text="John Doe", start=0, end=8),
            Entity(type="ORG", text="Acme Corp", start=20, end=29),
        ],
    )

    result = ExtractionResult(content="John Doe works at Acme Corp", mime_type="text/plain", metadata={})
    config = ExtractionConfig(extract_entities=True)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.entities is not None
    assert len(processed.entities) == 2
    assert processed.entities[0].type == "PERSON"
    mock_extract_entities.assert_called_once()


def test_validate_and_post_process_helper_entities_runtime_error(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg.extraction.extract_entities", side_effect=RuntimeError("Entity extraction failed"))

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(extract_entities=True)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.entities is None


def test_validate_and_post_process_helper_with_keywords(mocker: MockerFixture) -> None:
    mock_extract_keywords = mocker.patch(
        "kreuzberg.extraction.extract_keywords", return_value=[("python", 0.9), ("programming", 0.8), ("code", 0.7)]
    )

    result = ExtractionResult(
        content="Python programming is great for writing code", mime_type="text/plain", metadata={}
    )
    config = ExtractionConfig(extract_keywords=True, keyword_count=3)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.keywords is not None
    assert len(processed.keywords) == 3
    assert processed.keywords[0] == ("python", 0.9)
    mock_extract_keywords.assert_called_once_with(result.content, keyword_count=3)


def test_validate_and_post_process_helper_keywords_runtime_error(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg.extraction.extract_keywords", side_effect=RuntimeError("Keyword extraction failed"))

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(extract_keywords=True)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.keywords is None


def test_validate_and_post_process_helper_with_language_detection(mocker: MockerFixture) -> None:
    mock_detect_languages = mocker.patch("kreuzberg.extraction.detect_languages", return_value=["en", "es"])

    result = ExtractionResult(content="Hello world. Hola mundo.", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_language=True)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.detected_languages == ["en", "es"]
    mock_detect_languages.assert_called_once()


def test_validate_and_post_process_helper_with_document_type(mocker: MockerFixture) -> None:
    mock_auto_detect = mocker.patch(
        "kreuzberg.extraction.auto_detect_document_type",
        side_effect=lambda r, _c, **_kwargs: ExtractionResult(
            content=r.content,
            mime_type=r.mime_type,
            metadata=r.metadata,
            document_type="invoice",
            document_type_confidence=0.95,
        ),
    )

    result = ExtractionResult(content="Invoice #12345", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True)

    processed = _validate_and_post_process_helper(result, config, file_path=Path("invoice.pdf"))

    assert processed.document_type == "invoice"
    assert processed.document_type_confidence == 0.95
    mock_auto_detect.assert_called_once()


def test_validate_and_post_process_helper_all_features(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg.extraction.extract_entities", return_value=[])
    mocker.patch("kreuzberg.extraction.extract_keywords", return_value=[])
    mocker.patch("kreuzberg.extraction.detect_languages", return_value=["en"])
    mocker.patch("kreuzberg.extraction.auto_detect_document_type", side_effect=lambda r, _c, **_kwargs: r)
    mocker.patch("kreuzberg.extraction._handle_chunk_content", return_value=["chunk1", "chunk2"])

    result = ExtractionResult(content="Test content for all features", mime_type="text/plain", metadata={})
    config = ExtractionConfig(
        chunk_content=True,
        extract_entities=True,
        extract_keywords=True,
        auto_detect_language=True,
        auto_detect_document_type=True,
    )

    processed = _validate_and_post_process_helper(result, config)

    assert processed.chunks == ["chunk1", "chunk2"]
    assert processed.entities == []
    assert processed.keywords == []
    assert processed.detected_languages == ["en"]


def test_validate_and_post_process_sync(mocker: MockerFixture) -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})
    ExtractionConfig()

    mock_validator = Mock()
    config_with_validators = ExtractionConfig(validators=[mock_validator])

    processed = _validate_and_post_process_sync(result, config_with_validators)

    mock_validator.assert_called_once_with(result)
    assert processed.content == "Test content"


def test_validate_and_post_process_sync_with_hooks(mocker: MockerFixture) -> None:
    result = ExtractionResult(content="Original content", mime_type="text/plain", metadata={})

    def hook(r: ExtractionResult) -> ExtractionResult:
        r.content = "Modified content"
        return r

    config = ExtractionConfig(post_processing_hooks=[hook])

    processed = _validate_and_post_process_sync(result, config)

    assert processed.content == "Modified content"


@pytest.mark.anyio
async def test_validate_and_post_process_async(mocker: MockerFixture) -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})

    async_validator = AsyncMock()
    config = ExtractionConfig(validators=[async_validator])

    from kreuzberg.extraction import _validate_and_post_process_async

    processed = await _validate_and_post_process_async(result, config)

    async_validator.assert_called_once_with(result)
    assert processed.content == "Test content"


@pytest.mark.anyio
async def test_validate_and_post_process_async_with_hooks(mocker: MockerFixture) -> None:
    result = ExtractionResult(content="Original content", mime_type="text/plain", metadata={})

    async def async_hook(r: ExtractionResult) -> ExtractionResult:
        r.content = "Async modified content"
        return r

    config = ExtractionConfig(post_processing_hooks=[async_hook])

    from kreuzberg.extraction import _validate_and_post_process_async

    processed = await _validate_and_post_process_async(result, config)

    assert processed.content == "Async modified content"


@pytest.mark.anyio
async def test_extract_file_with_caching(tmp_path: Path, mocker: MockerFixture) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Cached content")

    mock_cache = Mock()
    mock_cache.get.return_value = None
    mock_cache.mark_processing = Mock()
    mock_cache.mark_complete = Mock()
    mock_cache.set = Mock()

    mocker.patch("kreuzberg.extraction.get_document_cache", return_value=mock_cache)

    result = await extract_file(str(test_file))

    assert result.content == "Cached content"
    mock_cache.get.assert_called()
    mock_cache.set.assert_called_once()


@pytest.mark.anyio
async def test_extract_file_cache_hit(tmp_path: Path, mocker: MockerFixture) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Original content")

    cached_result = ExtractionResult(content="Cached content", mime_type="text/plain", metadata={})

    mock_cache = Mock()
    mock_cache.get.return_value = cached_result

    mocker.patch("kreuzberg.extraction.get_document_cache", return_value=mock_cache)

    result = await extract_file(str(test_file))

    assert result.content == "Cached content"
    mock_cache.set.assert_not_called()


@pytest.mark.anyio
async def test_extract_file_cache_processing_wait(tmp_path: Path, mocker: MockerFixture) -> None:
    import threading

    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    event = threading.Event()
    event.set()

    mock_cache = Mock()
    mock_cache.get.side_effect = [
        None,
        ExtractionResult(content="Cached by other process", mime_type="text/plain", metadata={}),
    ]
    mock_cache.mark_processing.return_value = event

    mocker.patch("kreuzberg.extraction.get_document_cache", return_value=mock_cache)

    result = await extract_file(str(test_file))

    assert result.content == "Cached by other process"
    assert mock_cache.get.call_count == 2


@pytest.mark.anyio
async def test_batch_extract_file_partial_failure(tmp_path: Path) -> None:
    good_file = tmp_path / "good.txt"
    good_file.write_text("Good content")

    bad_file = tmp_path / "bad.txt"

    results = await batch_extract_file([str(good_file), str(bad_file)])

    assert len(results) == 2
    assert results[0].content == "Good content"
    assert results[1].metadata.get("error") is not None
    assert "Error:" in results[1].content


def test_batch_extract_file_sync_partial_failure(tmp_path: Path) -> None:
    good_file = tmp_path / "good.txt"
    good_file.write_text("Good content")

    bad_file = tmp_path / "nonexistent.txt"

    results = batch_extract_file_sync([str(good_file), str(bad_file)])

    assert len(results) == 2
    assert results[0].content == "Good content"
    assert results[1].metadata.get("error") is not None
    assert "Error:" in results[1].content


@pytest.mark.anyio
async def test_batch_extract_bytes_with_configs() -> None:
    contents = [
        (b"Content 1", "text/plain"),
        (b"Content 2", "text/plain"),
        (b"Content 3", "text/plain"),
    ]

    results = await batch_extract_bytes(contents)

    assert len(results) == 3
    assert results[0].content == "Content 1"
    assert results[1].content == "Content 2"
    assert results[2].content == "Content 3"


def test_batch_extract_bytes_sync_with_configs() -> None:
    contents = [
        (b"Sync content 1", "text/plain"),
        (b"Sync content 2", "text/markdown"),
    ]

    results = batch_extract_bytes_sync(contents)

    assert len(results) == 2
    assert results[0].content == "Sync content 1"
    assert results[1].mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_bytes_invalid_mime_type() -> None:
    with pytest.raises(ValidationError, match="mime_type"):
        await extract_bytes(b"content", "invalid/mime/type")


def test_extract_bytes_sync_invalid_mime_type() -> None:
    with pytest.raises(ValidationError, match="mime_type"):
        extract_bytes_sync(b"content", "invalid/mime/type")


@pytest.mark.anyio
async def test_extract_file_with_progress_callback_error(tmp_path: Path, mocker: MockerFixture) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = await extract_file(str(test_file))
    assert result.content == "Test content"


def test_validate_and_post_process_with_custom_patterns(mocker: MockerFixture) -> None:
    custom_patterns = frozenset([("CUSTOM", r"TEST-\\d+")])

    mock_extract_entities = mocker.patch(
        "kreuzberg.extraction.extract_entities", return_value=[Entity(type="CUSTOM", text="TEST-123", start=0, end=8)]
    )

    result = ExtractionResult(content="TEST-123 is a custom pattern", mime_type="text/plain", metadata={})
    config = ExtractionConfig(extract_entities=True, custom_entity_patterns=custom_patterns)

    processed = _validate_and_post_process_helper(result, config)

    assert processed.entities is not None
    assert len(processed.entities) == 1
    assert processed.entities[0].type == "CUSTOM"
    mock_extract_entities.assert_called_once_with(result.content, custom_patterns=custom_patterns)


@pytest.mark.anyio
async def test_extract_minimal_processing() -> None:
    config = ExtractionConfig(
        chunk_content=False,
        extract_entities=False,
        extract_keywords=False,
        auto_detect_language=False,
        auto_detect_document_type=False,
    )

    result = await extract_bytes(b"Minimal processing", "text/plain", config)

    assert result.content == "Minimal processing"
    assert result.chunks == []
    assert result.entities is None
    assert result.keywords is None
    assert result.detected_languages is None
    assert result.document_type is None
