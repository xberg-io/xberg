"""Integrity and applicability checks for local qualification artifacts."""

import hashlib
import json
import math
import shutil
from pathlib import Path


def _digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def snapshot(source: Path, directory: Path, label: str, *, executable: bool = False) -> dict:
    """Copy a mutable producer into a hash-bound read-only artifact."""
    digest = _digest(source)
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory / f"{label}-{digest}{source.suffix}"
    if not destination.exists():
        with source.open("rb") as incoming, destination.open("xb") as outgoing:
            shutil.copyfileobj(incoming, outgoing)
    if _digest(destination) != digest or _digest(source) != digest:
        raise ValueError("producer changed while creating snapshot")
    destination.chmod(0o555 if executable else 0o444)
    return {"file": str(destination.resolve()), "sha256": digest, "bytes": destination.stat().st_size}


def metric_applicability(result: dict, options: dict) -> dict:
    """Validate official metric results and preserve unsupported-TRM coverage."""
    if result.get("success") is not True:
        raise ValueError("official evaluator did not report success")
    values = {item["metric_name"]: item["value"] for item in result["metrics"]}
    for name in ("grits_con", "table_record_match"):
        if name not in values or not math.isfinite(values[name]) or not 0 <= values[name] <= 1:
            raise ValueError(f"missing or non-finite official metric: {name}")
    return {"grits_con": True, "table_record_match": not options.get("trm_unsupported", False)}


def _model_failure(text: str, source: str | None = None) -> bool:
    """Require model context, not renderer wording such as 'layout is preserved'."""
    model = source in {"layout", "pdf_tables", "native_pdf_layout", "layout-ocr"} or any(
        token in text
        for token in (
            "tatr",
            "rtdetr",
            "rt-detr",
            "table model",
            "onnx",
            "inference",
            "layout detection",
            "layout model",
            "layout engine",
            "layout runner",
            "layout_runner",
            "layout-for-markdown",
            "layout-based table recognition",
            "layout region plan",
            "xberg::layout:",
        )
    )
    failure = any(
        token in text
        for token in (
            "unavailable",
            "not available",
            "failed",
            "skipped",
            "fallback",
            "not compiled",
            "unsupported",
            "could not be loaded",
            "could not be prepared",
        )
    )
    return model and failure


def validate_diagnostics(result: dict, stderr: str, arm: str) -> None:
    """Reject actual model fallbacks while leaving other SDK warnings in their artifacts."""
    if arm == "native_html":
        return
    warnings = result.get("processing_warnings", [])
    if not isinstance(warnings, list):
        raise ValueError("invalid SDK processing warnings")
    for warning in warnings:
        text = f"{warning['source']} {warning['message']}".lower()
        if _model_failure(text, str(warning["source"]).lower()):
            raise ValueError(f"model fallback diagnostic: {text}")
    for line in stderr.lower().splitlines():
        if _model_failure(line):
            raise ValueError(f"model fallback diagnostic: {line}")


def validate_build(path: Path, commit: str, executable_sha256: str) -> None:
    """Bind required feature evidence to the exact frozen executable."""
    evidence = json.loads(path.read_text(encoding="utf-8"))
    if evidence["commit"] != commit or evidence["executable_sha256"] != executable_sha256:
        raise ValueError("build evidence does not match frozen producer")
    if not {"pdf", "ocr", "layout-detection"} <= set(evidence["features"]):
        raise ValueError("required PDF/OCR/layout features missing from build evidence")


def validate_invocation(invocation: dict) -> None:
    """Bind supported CLI extraction to the frozen PDF and configuration."""
    expected = ["extract", "{pdf}", "--config", "{config}", "--format", "json"]
    if invocation.get("argv") != expected:
        raise ValueError("use canonical extract argv bound to frozen PDF/config; overrides are forbidden")
    if invocation.get("content_path") != ["result", "content"]:
        raise ValueError("use the actual ExtractEnvelope result.content path")


def verify_bound_files(assets: list[dict]) -> None:
    """Verify live runtime paths against their frozen byte identities."""
    if not assets:
        raise ValueError("empty runtime artifact bindings")
    for asset in assets:
        if _digest(Path(asset["file"])) != asset["sha256"]:
            raise ValueError(f"bound artifact changed: {asset['file']}")


def _runtime_source_files(root: Path, imported: list[str]) -> dict[str, Path]:
    root = root.resolve()
    files = {str(path.relative_to(root)): path for path in sorted(root.rglob("*.py"))}
    if not {"runner.py", "contracts.py", "safeguards.py", "official.py"} <= files.keys():
        raise ValueError("incomplete runtime source inventory")
    for path in [*files.values(), *(Path(name) for name in imported)]:
        resolved = path.resolve()
        if not resolved.is_relative_to(root) or str(resolved.relative_to(root)) not in files:
            raise ValueError("imported runtime source does not match runner package")
    return files


def runtime_source_pins(root: Path, imported: list[str]) -> dict:
    """Bind the complete own Python inventory and the modules actually imported by the runner."""
    files = _runtime_source_files(root, imported)
    return {"version": 1, "files": {name: _digest(path) for name, path in files.items()}}


def verify_runtime_source_pins(root: Path, pins: dict | None, imported: list[str]) -> None:
    """Require a fresh source-bound plan before any extraction can execute."""
    if not isinstance(pins, dict) or pins.get("version") != 1 or not pins.get("files"):
        raise ValueError("frozen plan lacks supported runtime source pins; refreeze before execution")
    expected = pins["files"]
    if not isinstance(expected, dict):
        raise ValueError("invalid runtime source pins; refreeze before execution")
    files = {str(path.relative_to(root)): path for path in sorted(root.rglob("*.py"))}
    if set(expected) != set(files):
        raise ValueError("runtime source inventory changed; refreeze before execution")
    files = _runtime_source_files(root, imported)
    for name, path in files.items():
        _verify_runtime_source(name, path, expected[name])


def _verify_runtime_source(name: str, path: Path, digest: str) -> None:
    try:
        verify_bound_files([{"file": str(path), "sha256": digest}])
    except (OSError, ValueError) as error:
        raise ValueError(f"runtime source changed: {name}; refreeze before execution") from error
