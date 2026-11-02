"""Bind the complete local export before any ONNX graph is loaded."""

import hashlib
import json
import re
from pathlib import Path

COMPANIONS = (
    "config.json",
    "generation_config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
)
GRAPHS = ("encoder.onnx", "decoder.onnx")
ASSETS = (*COMPANIONS, *GRAPHS)


def unique_object(pairs: list[tuple]) -> dict:
    """Reject duplicate JSON keys rather than silently replacing a contract."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(path: Path) -> dict:
    """Read an object with an actionable filename on invalid metadata."""
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, UnicodeError, ValueError) as error:
        raise ValueError(f"invalid {path.name}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path.name} must contain a JSON object")
    return value


def digest(path: Path) -> str:
    """Stream learned weights without retaining another copy in memory."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def require_files(directory: Path, names: tuple[str, ...]) -> None:
    """Require each declared companion or graph at its actual runtime path."""
    for name in names:
        path = directory / name
        if not path.is_file() or path.resolve().parent != directory.resolve():
            raise ValueError(f"missing or nonlocal export asset: {name}")


def verify_integrity(directory: Path) -> None:
    """Verify all pins, including extra files, before parsing graph contents."""
    pins = read_json(directory / "sha256.json")
    for name in ASSETS:
        if name not in pins:
            raise ValueError(f"SHA-256 manifest missing {name}")
    for name, expected in pins.items():
        if Path(name).name != name or name == "sha256.json":
            raise ValueError(f"invalid SHA-256 asset path: {name}")
        if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise ValueError(f"invalid SHA-256 checksum for {name}")
        require_files(directory, (name,))
        if digest(directory / name) != expected:
            raise ValueError(f"SHA-256 checksum mismatch for {name}")
    actual = {str(path.relative_to(directory)) for path in directory.rglob("*") if path.is_file()}
    if actual != {*pins, "sha256.json"}:
        raise ValueError("SHA-256 manifest does not bind every export file")


def write_json(path: Path, value: dict) -> None:
    """Write deterministic metadata in the export directory only."""
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_manifest(directory: Path) -> None:
    """Pin both graphs and every required companion, with no model checkpoint."""
    require_files(directory, ASSETS)
    write_json(directory / "sha256.json", {name: digest(directory / name) for name in ASSETS})
