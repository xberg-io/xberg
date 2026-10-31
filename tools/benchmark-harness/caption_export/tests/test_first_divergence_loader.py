"""Exercise the real frozen-helper loader without model imports or external evidence."""

import hashlib
import importlib
import importlib.util
import os
import py_compile
import sys
from pathlib import Path

import pytest

SOURCE = 'PRODUCER = "source"\nSOURCE_FILE = __file__\nSOURCE_NAME = __name__\n'


def loader(monkeypatch):
    root = Path(__file__).resolve().parents[1]
    monkeypatch.syspath_prepend(str(root / "src"))
    try:
        module = importlib.import_module("caption_export.first_divergence_evidence")
    except ModuleNotFoundError as error:
        if error.name in {"caption_export", "caption_export.first_divergence_evidence"}:
            pytest.fail("actual first-divergence helper loader is missing", pytrace=False)
        raise
    assert Path(module.__file__).resolve().parent == root / "src" / "caption_export"
    monkeypatch.setattr(sys, "dont_write_bytecode", False)
    monkeypatch.setattr(sys, "pycache_prefix", None)
    return module.load_original_helper


def original_helper(tmp_path):
    directory = tmp_path / "original"
    directory.mkdir()
    path = directory / "parity.py"
    path.write_text(SOURCE)
    return path


def inventory(directory):
    return {str(path.relative_to(directory)): path.read_bytes() for path in directory.rglob("*") if path.is_file()}


def poison_matching_cache(source, tmp_path):
    poison = tmp_path / "poison.py"
    poison.write_text(SOURCE.replace('"source"', '"cached"'))
    stat = source.stat()
    os.utime(poison, ns=(stat.st_atime_ns, stat.st_mtime_ns))
    cache = Path(importlib.util.cache_from_source(str(source)))
    cache.parent.mkdir()
    py_compile.compile(
        str(poison),
        cfile=str(cache),
        dfile=str(source),
        doraise=True,
        invalidation_mode=py_compile.PycInvalidationMode.TIMESTAMP,
    )
    header = cache.read_bytes()[:16]
    assert int.from_bytes(header[4:8], "little") == 0, "real timestamp-based cache"
    assert int.from_bytes(header[8:12], "little") == int(stat.st_mtime) & 0xFFFFFFFF
    assert int.from_bytes(header[12:16], "little") == stat.st_size, "matching source size"
    return cache


def assert_source_identity(module, source):
    assert module.PRODUCER == "source", "execute pinned source, never the unpinned cached producer"
    assert module.__file__ == str(source)
    assert str(source) == module.SOURCE_FILE
    assert module.__name__ == module.SOURCE_NAME == "frozen_caption_parity"
    assert module.__spec__.name == module.__name__


@pytest.mark.parametrize("cached", [True, False], ids=["poisoned_matching_cache", "no_cache_writes"])
def test_should_execute_verified_helper_without_consuming_or_writing_original_cache(monkeypatch, tmp_path, cached):
    load = loader(monkeypatch)
    source = original_helper(tmp_path)
    pinned = hashlib.sha256(source.read_bytes()).hexdigest()
    if cached:
        cache = poison_matching_cache(source, tmp_path)
        assert cache.is_file(), "the wrong producer cache actually exists before loading"
    else:
        assert not (source.parent / "__pycache__").exists(), "original has no cache before loading"
    before = inventory(source.parent)
    module = load(source)
    assert_source_identity(module, source)
    assert hashlib.sha256(source.read_bytes()).hexdigest() == pinned
    assert inventory(source.parent) == before, "original file inventory and bytes must stay immutable"
    if not cached:
        assert not (source.parent / "__pycache__").exists(), "loading must not create original cache directories"
