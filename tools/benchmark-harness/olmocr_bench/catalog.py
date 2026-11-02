"""Read pinned corpus records without copying source documents into Git."""

import json
from collections import Counter
from pathlib import Path

from .contracts import (
    DATASET_REVISION,
    EVALUATOR_REVISION,
    GROUP_COUNTS,
    PDF_COUNT,
    RAW_TEST_COUNT,
    TEST_COUNT,
    digest,
    inject_baselines,
    verify_assets,
)


def file_binding(path: Path) -> dict:
    """Bind the live path, including cache symlinks, rather than an obsolete target."""
    return {"file": str(path.absolute()), "sha256": digest(path)}


def staged_file(root: Path, relative: str) -> Path:
    """Keep source manifest paths inside their explicitly chosen corpus root."""
    path = root / relative
    if Path(relative).is_absolute() or not path.resolve().is_relative_to(root.resolve()):
        raise ValueError("stage path must stay relative to corpus root")
    return path


def _registry_stage(registry: Path) -> dict:
    metadata = json.loads(registry.read_text())
    if metadata.get("format") != "xberg-benchmark-upstreams" or metadata.get("schema_version") != 1:
        raise ValueError("unsupported upstream registry format")
    source_id = "olmocr-issue2331"
    source = metadata["sources"][source_id]
    if source["dataset_revision"] != DATASET_REVISION or source["evaluator_revision"] != EVALUATOR_REVISION:
        raise ValueError("pinned registry revision mismatch")
    selected = [
        (key.removeprefix(source_id + "/"), value)
        for key, value in metadata["objects"].items()
        if value["source"] == source_id
    ]
    pdfs = [
        {"id": value["id"], "file": key, "sha256": value["sha256"]} for key, value in selected if value["kind"] == "pdf"
    ]
    tests = [
        {"group": value["group"], "file": key, "sha256": value["sha256"]}
        for key, value in selected
        if value["kind"] == "jsonl"
    ]
    if source["pdf_count"] != PDF_COUNT or source["jsonl_count"] != 5 or len(tests) != 5:
        raise ValueError("registry corpus count mismatch")
    return {
        "dataset_revision": source["dataset_revision"],
        "evaluator_revision": source["evaluator_revision"],
        "records": pdfs,
        "test_files": tests,
    }


def load_catalog(root: Path, registry: Path | None = None) -> dict:
    """Verify all 984 PDFs and all test references before scheduling work."""
    descriptor = registry or root / "stage.json"
    stage = _registry_stage(registry) if registry else json.loads(descriptor.read_text())
    if stage["dataset_revision"] != DATASET_REVISION or stage["evaluator_revision"] != EVALUATOR_REVISION:
        raise ValueError("pinned source revision mismatch")
    pdfs = stage["records"]
    if len(pdfs) != PDF_COUNT or len({row["id"] for row in pdfs}) != PDF_COUNT:
        raise ValueError("expected 984 unique PDFs")
    assets = []
    for row in pdfs:
        path = staged_file(root, row["file"])
        assets.append({"file": str(path), "sha256": row["sha256"]})
    definitions = []
    for entry in stage["test_files"]:
        path = staged_file(root, entry["file"])
        assets.append({"file": str(path), "sha256": entry["sha256"]})
        definitions.extend(dict(json.loads(line), group=entry["group"]) for line in path.read_text().splitlines())
    verify_assets(assets)
    if len(definitions) != RAW_TEST_COUNT:
        raise ValueError("expected 5113 raw tests")
    pdf_names = [row["file"].removeprefix("bench_data/pdfs/") for row in pdfs]
    if len(set(pdf_names)) != PDF_COUNT or {row["pdf"] for row in definitions} != set(pdf_names):
        raise ValueError("PDF reference coverage mismatch")
    if any(row["page"] != 1 for row in definitions):
        raise ValueError("unexpected page index in frozen corpus")
    tests = inject_baselines(definitions, pdf_names)
    if len(tests) != TEST_COUNT or dict(Counter(row["group"] for row in tests)) != GROUP_COUNTS:
        raise ValueError("official test/baseline census mismatch")
    return {
        "pdfs": pdfs,
        "tests": tests,
        "input_assets": assets,
        "descriptor": file_binding(descriptor),
        "registry": str(registry.absolute()) if registry else None,
    }
