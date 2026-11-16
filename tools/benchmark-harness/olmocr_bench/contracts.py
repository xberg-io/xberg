"""Complete-corpus contracts; synthetic integrity inputs never establish OCR quality."""

import hashlib
import shutil
from pathlib import Path

PDF_COUNT = 984
RAW_TEST_COUNT = 5113
TEST_COUNT = 6090
GROUP_COUNTS = {
    "arxiv_math": 2927,
    "old_scans_math": 458,
    "old_scans": 526,
    "long_tiny_text": 442,
    "headers_footers": 760,
    "baseline": 977,
}
ROLES = ("baseline", "candidate")
DATASET_REVISION = "54a96a6fb6a2bd3b297e59869491db4d3625b711"
EVALUATOR_REVISION = "f7cfe4c22098b154c76b6ec950d1c0a464eecf8d"

# RapidLaTeXOCR v0.0.0 pins enforced by the native formula recognizer.
FORMULA_MODEL_SHA256 = {
    "image_resizer.onnx": "e0b075c39700f64d50400f39c8fc186bbb3b5d84d31864008313f376603aca9d",
    "encoder.onnx": "01bf5dc25539ca0cd5b1bd29296ea495977a6ba5f629dc4178277809d26e5e7d",
    "decoder.onnx": "bd695497bf1b22279b7626f5916c79226e1e244c84355f8da7edfd2d921d0072",
    "tokenizer.json": "1dc27b18d6a518d0d5ff3f4bb7bd98521fe80ad39e5b2a246d4109f1bb9d5019",
}


def formula_model_paths(models: list[dict]) -> dict[str, Path]:
    """Identify exactly one integrity-bound path for each native formula asset."""
    paths = [(Path(row["file"]), row["sha256"]) for row in models]
    selected = {}
    for name, sha256 in FORMULA_MODEL_SHA256.items():
        matches = [path for path, digest in paths if path.name == name and digest == sha256]
        if len(matches) != 1 or not matches[0].is_absolute():
            raise ValueError(f"expected exactly one pinned RapidLaTeXOCR {name} at an absolute path")
        selected[name] = matches[0]
    return selected


def digest(path: Path) -> str:
    """Hash exact artifact bytes without loading large models into memory."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inject_baselines(rows: list[dict], pdfs: list[str]) -> list[dict]:
    """Mirror the pinned official CLI's per-PDF baseline injection exactly."""
    if len({row["id"] for row in rows}) != len(rows):
        raise ValueError("duplicate test ID")
    if any(row["pdf"] not in pdfs for row in rows):
        raise ValueError("unresolved PDF reference")
    result = list(rows)
    baselined = {row["pdf"] for row in rows if row["type"] == "baseline"}
    result.extend(
        {"id": f"{pdf}_baseline", "pdf": pdf, "page": 1, "type": "baseline", "group": "baseline"}
        for pdf in pdfs
        if pdf not in baselined
    )
    if len({row["id"] for row in result}) != len(result):
        raise ValueError("duplicate test ID after baseline injection")
    return result


def validate_config(config: dict) -> None:
    """Require actual native Markdown layout/formula activation and disabled caches."""
    if config.get("output_format") != "markdown" or config.get("use_cache") is not False:
        raise ValueError("Markdown and disabled extraction cache required")
    if config.get("use_layout_for_markdown") is not True:
        raise ValueError("use_layout_for_markdown=true required for native PDF activation")
    layout = config.get("layout") or {}
    if layout.get("formula_model") != "latex_ocr":
        raise ValueError("explicit latex_ocr formula model required")
    if layout.get("strategy") != "always" or layout.get("acceleration", {}).get("provider") != "cpu":
        raise ValueError("explicit always strategy and CPU acceleration required")
    ocr = config.get("ocr", {}).get("tesseract_config", {})
    if ocr.get("use_cache") is not False or not ocr.get("language"):
        raise ValueError("explicit nested OCR cache disable and language required")
    if config.get("disable_ocr") is not False:
        raise ValueError("OCR must be enabled for scanned-page qualification")
    threads = config.get("concurrency", {}).get("max_threads")
    if type(threads) is not int or threads < 1:
        raise ValueError("positive CPU thread budget required")


def validate_argv(argv: list[str]) -> None:
    """Bind CLI extraction to the scheduled PDF and immutable config."""
    if argv != ["extract", "{pdf}", "--config", "{config}", "--format", "json"]:
        raise ValueError("canonical CLI invocation required; overrides forbidden")


def verify_assets(assets: list[dict]) -> None:
    """Detect changed bytes at the actual runtime paths, preserving symlink bindings."""
    if not assets:
        raise ValueError("empty artifact bindings")
    for asset in assets:
        if digest(Path(asset["file"])) != asset["sha256"]:
            raise ValueError(f"artifact changed: {asset['file']}")


def schedule(pdfs: list[dict]) -> list[dict]:
    """Schedule each scoped PDF once for each producer."""
    if not pdfs or len({row["id"] for row in pdfs}) != len(pdfs):
        raise ValueError("empty or duplicate PDF schedule")
    return [{"id": f"{role}/{pdf['id']}", "role": role, "pdf_id": pdf["id"]} for role in ROLES for pdf in pdfs]


def validate_results(cases: list[dict], rows: list[dict]) -> None:
    """Reject zero work, missing cases and duplicated or unknown results."""
    ids = [row["id"] for row in rows]
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate result")
    if not cases or {case["id"] for case in cases} != set(ids):
        raise ValueError("missing or unexpected results")


def snapshot(path: Path, directory: Path, label: str, *, executable: bool = False) -> dict:
    """Copy a mutable producer to a hash-bound read-only artifact."""
    checksum = digest(path)
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / f"{label}-{checksum}{path.suffix}"
    if not target.exists():
        with path.open("rb") as source, target.open("xb") as output:
            shutil.copyfileobj(source, output)
    if digest(target) != checksum or digest(path) != checksum:
        raise ValueError("producer changed while snapshotting")
    target.chmod(0o555 if executable else 0o444)
    return {"file": str(target.resolve()), "sha256": checksum}


def verify_predictions(root: Path, assets: list[dict]) -> None:
    """Reject extra repeats, swapped files and changed native outputs before scoring."""
    expected = {Path(row["file"]).absolute() for row in assets}
    if {path.absolute() for path in root.rglob("*.md")} != expected:
        raise ValueError("prediction inventory differs from frozen one-repeat coverage")
    if assets:
        verify_assets(assets)
