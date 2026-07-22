"""Tests for _base module helpers."""

from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import MagicMock

from xberg import ExtractionResult

from xberg_surrealdb._base import (
    _collect_files,
    _content_hash,
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
    dt = datetime(2024, 1, 1, tzinfo=timezone.utc)
    assert _parse_datetime(dt) is dt


def test_parse_datetime_parses_iso_string_with_tz() -> None:
    result = _parse_datetime("2024-01-01T00:00:00+00:00")
    assert isinstance(result, datetime)
    assert result.tzinfo is not None


def test_parse_datetime_adds_utc_to_naive_iso_string() -> None:
    result = _parse_datetime("2024-01-01T00:00:00")
    assert isinstance(result, datetime)
    assert result.tzinfo == timezone.utc


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
    result = MagicMock(spec=ExtractionResult)
    result.content = "test content"
    result.mime_type = "text/plain"
    result.metadata = {}
    result.quality_score = None
    result.detected_languages = []
    result.extracted_keywords = []

    doc = _map_result_to_doc(result, "source.txt", "documents")

    assert doc["title"] is None
    assert doc["authors"] is None
    assert doc["created_at"] is None
