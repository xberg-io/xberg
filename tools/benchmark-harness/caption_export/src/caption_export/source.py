"""Verify an explicitly selected local checkpoint without fetching remote code."""

import re
from pathlib import Path

from .assets import digest, read_json

REQUIRED = {
    "config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
}
CHECKPOINTS = {"model.safetensors", "pytorch_model.bin"}


def inventory_files(inventory: dict) -> dict[str, dict]:
    """Reject incomplete, duplicate or nonlocal file declarations."""
    records = inventory.get("source_files")
    if not isinstance(records, list) or not records:
        raise ValueError("source inventory requires nonempty source_files")
    files = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("invalid source inventory file record")
        name = record.get("file")
        if not isinstance(name, str) or Path(name).name != name or name in (".", "..") or name in files:
            raise ValueError("duplicate or nonlocal source inventory path")
        if name.endswith(".py"):
            raise ValueError("remote Python is not a supported source asset")
        if type(record.get("bytes")) is not int or record["bytes"] <= 0:
            raise ValueError(f"source inventory requires positive size: {name}")
        if not isinstance(record.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", record["sha256"]):
            raise ValueError(f"source inventory requires full sha256: {name}")
        files[name] = record
    if not REQUIRED.issubset(files) or len(CHECKPOINTS & files.keys()) != 1:
        raise ValueError("source inventory requires metadata and exactly one supported checkpoint")
    return files


def verify_source(source: Path, receipt: Path, repository: str, revision: str) -> dict:
    """Bind exact identity and all bytes before any model is deserialized."""
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("source revision must be a full lowercase 40-character commit")
    inventory = read_json(receipt)
    if inventory.get("source_repository") != repository or inventory.get("source_revision") != revision:
        raise ValueError("source repository/revision differs from verified inventory")
    files = inventory_files(inventory)
    actual = {p.name for p in source.iterdir()}
    if actual != files.keys():
        raise ValueError("source inventory does not bind the exact local file set")
    for name, record in files.items():
        path = source / name
        if path.is_symlink() or not path.is_file() or path.resolve().parent != source.resolve():
            raise ValueError(f"source inventory asset is not a local regular file: {name}")
        if digest(path) != record["sha256"] or path.stat().st_size != record["bytes"]:
            raise ValueError(f"source sha256/size mismatch: {name}")
    return inventory
