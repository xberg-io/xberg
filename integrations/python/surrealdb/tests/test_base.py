"""Tests for _base module helpers."""

from datetime import UTC, datetime
from pathlib import Path

import pytest
from xberg import ExtractedDocument
from xberg._xberg import Metadata

from surrealdb_xberg._base import (
    _collect_files,
    _content_hash,
    _guess_mime_type,
    _map_result_to_doc,
    _parse_datetime,
)


def test_content_hash_is_deterministic() -> None:
    assert _content_hash("hello") == _content_hash("hello")


def test_content_hash_differs_for_different_content() -> None:
    assert _content_hash("hello") != _content_hash("world")


def test_parse_datetime_none_returns_none() -> None:
    assert _parse_datetime(None) is None


def test_parse_datetime_returns_datetime_object_as_is() -> None:
    dt = datetime(2024, 1, 1, tzinfo=UTC)
    assert _parse_datetime(dt) is dt


def test_parse_datetime_parses_iso_string_with_tz() -> None:
    result = _parse_datetime("2024-01-01T00:00:00+00:00")
    assert isinstance(result, datetime)
    assert result.tzinfo is not None


def test_parse_datetime_adds_utc_to_naive_iso_string() -> None:
    result = _parse_datetime("2024-01-01T00:00:00")
    assert isinstance(result, datetime)
    assert result.tzinfo == UTC


def test_parse_datetime_invalid_string_returns_none() -> None:
    assert _parse_datetime("not-a-date") is None


def test_parse_datetime_non_string_non_datetime_returns_none() -> None:
    assert _parse_datetime(12345) is None


async def test_collect_files_finds_matching_files(tmp_path: Path) -> None:
    (tmp_path / "a.txt").write_text("a")
    (tmp_path / "b.txt").write_text("b")
    (tmp_path / "c.md").write_text("c")

    result = await _collect_files(tmp_path, "*.txt")
    assert len(result) == 2
    assert all(p.suffix == ".txt" for p in result)


async def test_collect_files_returns_sorted(tmp_path: Path) -> None:
    (tmp_path / "z.txt").write_text("z")
    (tmp_path / "a.txt").write_text("a")

    result = await _collect_files(tmp_path, "*.txt")
    assert result == sorted(result)


async def test_collect_files_skips_directories(tmp_path: Path) -> None:
    (tmp_path / "sub").mkdir()
    (tmp_path / "file.txt").write_text("f")

    result = await _collect_files(tmp_path, "*")
    assert len(result) == 1
    assert result[0].name == "file.txt"


async def test_collect_files_empty_directory(tmp_path: Path) -> None:
    assert await _collect_files(tmp_path, "*.txt") == []


def test_map_result_to_doc_handles_missing_metadata_keys() -> None:
    document = ExtractedDocument(
        content="test content",
        mime_type="text/plain",
        metadata=Metadata(),
        detected_languages=[],
        chunks=[],
        extracted_keywords=[],
        quality_score=None,
    )

    doc = _map_result_to_doc(document, "source.txt", "documents")

    assert doc["title"] is None
    assert doc["authors"] is None
    assert doc["created_at"] is None
    assert doc["quality_score"] is None
    assert doc["detected_languages"] == []
    assert doc["keywords"] == []


def test_map_result_to_doc_maps_metadata_and_keywords() -> None:
    from tests.conftest import make_document

    document = make_document()
    doc = _map_result_to_doc(document, "source.txt", "documents")

    assert doc["title"] == "Test Document"
    assert doc["authors"] == "Alice, Bob"
    assert doc["detected_languages"] == ["en"]
    assert doc["keywords"] == ["test"]
    assert doc["created_at"] is not None
    assert doc["metadata"]["title"] == "Test Document"
    assert doc["metadata"]["authors"] == ["Alice", "Bob"]


def test_map_result_to_doc_no_entities_tables_summary_defaults() -> None:
    document = ExtractedDocument(
        content="test content",
        mime_type="text/plain",
        metadata=Metadata(),
        detected_languages=[],
        chunks=[],
        extracted_keywords=[],
        quality_score=None,
    )

    doc = _map_result_to_doc(document, "source.txt", "documents")

    assert doc["summary"] is None
    assert doc["entities"] == []
    assert doc["tables"] == []


def test_map_result_to_doc_persists_entities_tables_summary() -> None:
    from tests.conftest import make_document, make_entity, make_table

    document = make_document(
        entities=[make_entity("person", "Alice", 0, 5, 0.87)],
        tables=[make_table("|a|b|\n|-|-|\n|1|2|", 3, [["a", "b"], ["1", "2"]])],
        summary="A concise abstractive summary.",
    )

    doc = _map_result_to_doc(document, "source.pdf", "documents")

    assert doc["summary"] == "A concise abstractive summary."
    assert doc["entities"] == [
        {"category": "person", "text": "Alice", "start": 0, "end": 5, "confidence": pytest.approx(0.87)},
    ]
    assert doc["tables"] == [
        {"markdown": "|a|b|\n|-|-|\n|1|2|", "page_number": 3, "cells": [["a", "b"], ["1", "2"]]},
    ]


def test_guess_mime_type_known_and_unknown() -> None:
    assert _guess_mime_type("report.pdf") == "application/pdf"
    assert _guess_mime_type("notes.txt") == "text/plain"
    assert _guess_mime_type("mystery.unknownext") == "application/octet-stream"
