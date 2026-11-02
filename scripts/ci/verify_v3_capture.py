"""Reject empty V3 capture commands and incomplete tensor artifacts."""

from __future__ import annotations

import hashlib
import json
import re
import struct
import sys
from pathlib import Path

INPUT_NAMES = {"im_shape", "image", "scale_factor"}
OUTPUT_NAMES = {"fetch_name_0", "fetch_name_1", "fetch_name_2"}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_scope(listing: Path, expected: str) -> None:
    names = re.findall(r"^(.+): test$", listing.read_text(), re.MULTILINE)
    require(names == [expected], "expected exactly one capture test")


def tensor_record(root: Path, page: str, side: str, record: dict, seen: set[Path]) -> tuple[list[int], Path]:
    filename = record["file"]
    require(
        isinstance(filename, str) and filename not in {"", ".", ".."} and not re.search(r"[/\\:]", filename),
        "invalid tensor artifact path",
    )
    path = root / page / side / filename
    resolved = path.resolve(strict=True)
    require(resolved.is_relative_to(root.resolve()) and resolved not in seen, "escaped or duplicate tensor artifact")
    require(path.is_file() and not path.is_symlink(), "tensor artifact must be a regular file")
    seen.add(resolved)
    shape = record["shape"]
    require(isinstance(shape, list) and shape and all(type(n) is int and n > 0 for n in shape), "invalid tensor shape")
    require(record["dtype"] in {"<f4", "<i4"}, "invalid tensor dtype")
    expected_bytes = 4
    for dimension in shape:
        expected_bytes *= dimension
    require(
        type(record["bytes"]) is int and record["bytes"] == expected_bytes == path.stat().st_size,
        "tensor size mismatch",
    )
    digest = record["sha256"]
    require(isinstance(digest, str) and re.fullmatch(r"[0-9a-f]{64}", digest) is not None, "invalid tensor digest")
    with path.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == digest, "tensor digest mismatch")
    return shape, path


def validate_row(root: Path, row: dict, seen: set[Path]) -> None:
    frames = {}
    for side, names in [("inputs", INPUT_NAMES), ("outputs", OUTPUT_NAMES)]:
        records = row[side]
        require(isinstance(records, list) and len(records) == 3, "expected three named tensors per side")
        require({r["name"] for r in records} == names, "missing or duplicate named tensors")
        frames[side] = {r["name"]: (r, *tensor_record(root, row["input"], side, r, seen)) for r in records}
    for name, shape in [("image", [1, 3, 800, 800]), ("im_shape", [1, 2]), ("scale_factor", [1, 2])]:
        record, actual, _ = frames["inputs"][name]
        require(actual == shape and record["dtype"] == "<f4", "invalid input frame")
    boxes, box_shape, _ = frames["outputs"]["fetch_name_0"]
    count, count_shape, count_path = frames["outputs"]["fetch_name_1"]
    masks, mask_shape, _ = frames["outputs"]["fetch_name_2"]
    require(len(box_shape) == 2 and box_shape[1] == 7 and boxes["dtype"] == "<f4", "invalid box frame")
    require(count_shape == [1] and count["dtype"] == "<i4", "invalid count frame")
    require(mask_shape == [box_shape[0], 200, 200] and masks["dtype"] == "<i4", "invalid mask frame")
    require(struct.unpack("<i", count_path.read_bytes())[0] == box_shape[0], "output count and rows disagree")


def validate_capture(runtime: Path, root: Path) -> None:
    outcomes = re.findall(
        r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", runtime.read_text(), re.MULTILINE
    )
    require(outcomes == [("1", "0", "0")], "capture test did not execute exactly once")
    manifest = root / "manifest.json"
    require(manifest.is_file(), "capture manifest missing")
    try:
        require(root.is_absolute() and not root.is_symlink() and not manifest.is_symlink(), "invalid capture root")
        data = json.loads(manifest.read_text())
        require(data["schema_version"] == 1 and data["qualification_complete"] is False, "invalid capture status")
        require(data["real_forward_passes"] == 2, "expected two real forwards")
        require(data["input_tensors"] == data["output_tensors"] == 6, "expected six tensors per direction")
        rows = data["rows"]
        require(
            isinstance(rows, list) and [r["input"] for r in rows] == ["document-1", "document-2"],
            "invalid page coverage",
        )
        seen: set[Path] = set()
        for row in rows:
            validate_row(root, row, seen)
        require(len(seen) == 12, "expected twelve distinct tensor artifacts")
    except (KeyError, TypeError, ValueError, OSError, struct.error) as error:
        raise ValueError(f"capture manifest incomplete: {error}") from error


def main(arguments: list[str]) -> None:
    if len(arguments) != 3 or arguments[0] not in {"scope", "capture"}:
        raise ValueError("usage: verify_v3_capture.py scope LISTING TEST | capture RUNTIME OUTPUT")
    if arguments[0] == "scope":
        validate_scope(Path(arguments[1]), arguments[2])
    else:
        validate_capture(Path(arguments[1]), Path(arguments[2]))


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (ValueError, OSError) as error:
        print(f"V3 capture validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
