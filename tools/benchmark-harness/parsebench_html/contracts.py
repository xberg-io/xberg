"""Artifact contracts for the fixed ParseBench HTML comparison."""

import hashlib
import json
from collections import Counter
from pathlib import Path

DATASET_REVISION = "2805a1d940f95a203e0ae4b88be9934f7765b3fc"
EVALUATOR_REVISION = "328c71109e0b955f66583c33ec95ac52d9749db4"
RECORD_COUNT = 503
ARMS = ("native_html", "layout_tatr_html", "forced_ocr_layout_html")
ROLES = ("baseline", "candidate")


def sha256(path: Path) -> str:
    """Hash an artifact without loading it into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verified_file(root: Path, record: dict) -> Path:
    """Verify a safe relative artifact path, size and checksum."""
    relative = Path(record["file"])
    path = (root / relative).resolve()
    if relative.is_absolute() or not path.is_relative_to(root.resolve()):
        raise ValueError("artifact path must be relative and inside stage")
    if sha256(path) != record["sha256"]:
        raise ValueError(f"digest mismatch: {relative}")
    if path.stat().st_size != record["bytes"]:
        raise ValueError(f"size mismatch: {relative}")
    return path


def read_stage(root: Path, expected_records: int = RECORD_COUNT) -> dict:
    """Verify the complete pinned stage before any extraction is scheduled."""
    value = json.loads((root / "stage.json").read_text(encoding="utf-8"))
    records = value["records"]
    if not expected_records or len(records) != expected_records:
        raise ValueError("record count does not match frozen corpus")
    if len({row["id"] for row in records}) != len(records):
        raise ValueError("duplicate record")
    if value["dataset_revision"] != DATASET_REVISION or value["evaluator_revision"] != EVALUATOR_REVISION:
        raise ValueError("revision mismatch")
    for row in records:
        verified_file(root, row["pdf"])
        oracle = verified_file(root, row["oracle"]).read_text(encoding="utf-8")
        if "<table" not in oracle.lower():
            raise ValueError("missing raw HTML table oracle")
    return value


def schedule(records: list[dict], roles: list[str], arms: list[str]) -> list[dict]:
    """Enumerate every role, arm and document exactly once."""
    if tuple(roles) != ROLES or tuple(arms) != ARMS:
        raise ValueError("roles and arms must match complete frozen comparison")
    if not records or len({row["id"] for row in records}) != len(records):
        raise ValueError("empty or duplicate records")
    return [
        {"id": f"{role}/{arm}/{row['id']}", "role": role, "arm": arm, "record_id": row["id"]}
        for role in roles
        for arm in arms
        for row in records
    ]


def validate_rows(cases: list[dict], rows: list[dict]) -> dict:
    """Reject missing, duplicated or unexpected scheduled results."""
    expected = {case["id"] for case in cases}
    seen = set()
    for row in rows:
        if row["id"] in seen:
            raise ValueError("duplicate result")
        if row["id"] not in expected:
            raise ValueError("unexpected result")
        seen.add(row["id"])
    if expected != seen or not expected:
        raise ValueError("missing result")
    for row in rows:
        if row["status"] not in {"predicted", "empty_prediction", "extraction_error"}:
            raise ValueError("invalid result status")
        if row["status"] != "extraction_error":
            if not row.get("prediction"):
                raise ValueError("missing prediction artifact")
            if (
                row.get("evaluation_status") == "evaluated"
                and not {"grits_con", "table_record_match"} <= row.get("metrics", {}).keys()
            ):
                raise ValueError("missing official metrics")
            if row.get("evaluation_status") == "evaluated" and row.get("official_success") is not True:
                raise ValueError("missing official success")
    return {"count": len(rows), "statuses": dict(Counter(row["status"] for row in rows))}


def validate_config(config: dict, arm: str) -> None:
    """Require HTML output and explicit cache isolation for each arm."""
    if arm not in ARMS:
        raise ValueError("unknown arm")
    if config.get("output_format") != "html" or config.get("use_cache") is not False:
        raise ValueError("HTML output and disabled extraction cache required")
    ocr_cache = config.get("ocr", {}).get("tesseract_config", {}).get("use_cache")
    if ocr_cache is not False:
        raise ValueError("OCR cache must be explicitly disabled")
    if arm == "native_html":
        if config.get("layout") is not None or config.get("disable_ocr") is not True or config.get("force_ocr", False):
            raise ValueError("native arm requires disabled OCR and layout")
    elif not isinstance(config.get("layout"), dict) or config["layout"].get("table_model") != "tatr":
        raise ValueError("layout arms require explicit TATR")
    elif config["layout"].get("strategy") != "always":
        raise ValueError("model arms require explicit strategy=always")
    elif config["layout"].get("acceleration", {}).get("provider") != "cpu":
        raise ValueError("model arms require explicit CPU acceleration")
    elif (
        not isinstance(config.get("concurrency", {}).get("max_threads"), int)
        or config["concurrency"]["max_threads"] < 1
    ):
        raise ValueError("model arms require positive CPU thread budget")
    if arm == "layout_tatr_html" and config.get("use_layout_for_markdown") is not True:
        raise ValueError("native layout arm requires use_layout_for_markdown=true")
    if arm == "forced_ocr_layout_html":
        if config.get("force_ocr") is not True or config.get("disable_ocr", False):
            raise ValueError("forced OCR arm requires force_ocr and enabled OCR")
    elif config.get("force_ocr", False):
        raise ValueError("force_ocr is only allowed in forced OCR arm")
