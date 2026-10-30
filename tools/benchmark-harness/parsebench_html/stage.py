"""Stage pinned ParseBench inputs without downloading or normalizing ground truth."""

import argparse
import hashlib
import importlib
import json
import re
import shutil
import sys
from pathlib import Path
from types import ModuleType
from typing import Protocol

from contracts import DATASET_REVISION, EVALUATOR_REVISION, RECORD_COUNT, read_stage, sha256

SOURCE = "parsebench-tables"
DATASET = "llamaindex/ParseBench"
EVALUATOR = "run-llama/ParseBench"
JSONL_SHA256 = "66129fcc9e68ae0bf40e7d41f540002ceff9bedf46d6dafff2321ba2b7ab5231"


class AssetPin(Protocol):
    """Fields supplied by the public corpus acquisition descriptor."""

    path: str
    sha256: str
    size: int
    kind: str
    upstream_path: str
    record_id: str | None


def _helpers(corpus_root: Path) -> ModuleType:
    scripts = (corpus_root / "scripts").resolve(strict=True)
    sys.path.insert(0, str(scripts))
    module = importlib.import_module("corpus_tools.benchmark_upstreams")
    if not Path(module.__file__).resolve().is_relative_to(scripts):
        raise ValueError("corpus helper was imported from a different corpus root")
    return module


def _verified_inputs(helper: ModuleType, registry: Path, cache: Path) -> tuple[list[AssetPin], dict[str, AssetPin]]:
    source = helper.SOURCES[SOURCE]
    actual = tuple(source.get(key) for key in ("dataset", "dataset_revision", "evaluator", "evaluator_revision"))
    if actual != (DATASET, DATASET_REVISION, EVALUATOR, EVALUATOR_REVISION):
        raise ValueError("corpus helper differs from adapter frozen source identities and revisions")
    pins = [pin for pin in helper.load_inventory(registry) if pin.source == SOURCE]
    pdfs = [pin for pin in pins if pin.kind == "pdf"]
    jsonls = [pin for pin in pins if pin.kind == "jsonl"]
    if len(pdfs) != RECORD_COUNT or len(jsonls) != 1:
        raise ValueError("expected exactly 503 PDF pins and one JSONL pin")
    if jsonls[0].sha256 != JSONL_SHA256:
        raise ValueError("pinned ParseBench JSONL checksum differs")
    _verify_cache(helper, pins, cache)
    return pins, {pin.upstream_path: pin for pin in pdfs}


def _verify_cache(helper: ModuleType, pins: list[AssetPin], cache: Path) -> None:
    for pin in pins:
        status = helper.fetch_one(pin, cache, verify_only=True)
        if status != helper.STATUS_SKIPPED:
            raise ValueError(f"cached upstream pin failed: {pin.path}: {status}")


def _raw_rows(path: Path, pdfs: dict[str, AssetPin]) -> list[dict]:
    rows = []
    ids = set()
    paths = set()
    lines = path.read_text(encoding="utf-8").splitlines()
    if len(lines) != RECORD_COUNT:
        raise ValueError("raw record count must be exactly 503")
    for line in lines:
        row = json.loads(line, object_pairs_hook=_unique_object)
        if not isinstance(row, dict) or not isinstance(row.get("id"), str) or not row["id"]:
            raise ValueError("invalid raw record identity")
        upstream = row.get("pdf")
        if not isinstance(upstream, str) or upstream not in pdfs:
            raise ValueError("raw PDF path does not match a pinned PDF")
        record_id = hashlib.sha256(row["id"].encode()).hexdigest()[:20]
        if record_id in ids or upstream in paths:
            raise ValueError("duplicate raw record identity or PDF")
        if record_id != pdfs[upstream].record_id:
            raise ValueError("raw record identity does not match PDF pin")
        _validate_oracle(row)
        ids.add(record_id)
        paths.add(upstream)
        rows.append(row)
    if len(rows) != RECORD_COUNT or paths != pdfs.keys():
        raise ValueError("raw record count must cover all 503 pinned PDFs")
    return rows


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def _validate_oracle(row: dict) -> None:
    oracle = row.get("expected_markdown")
    if not isinstance(oracle, str) or "<table" not in oracle.lower():
        raise ValueError("raw oracle must contain an HTML table")
    rule = row.get("rule")
    if not isinstance(rule, str) or not isinstance(json.loads(rule, object_pairs_hook=_unique_object), dict):
        raise ValueError("raw evaluation rule must encode a JSON object")
    if row.get("category") != "table" or row.get("type") != "expected_markdown":
        raise ValueError("unexpected raw ParseBench table record type")


def _curated(path: Path | None) -> dict:
    if path is None:
        return {}
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object)
    rows = [row for row in value["documents"] if row.get("source_dataset") == "parsebench"]
    result = {row["id"]: row for row in rows}
    if len(result) != len(rows):
        raise ValueError("duplicate curated metadata identity")
    return result


def _curation(row: dict, curated: dict, oracle_sha: str) -> dict:
    key = "pb_" + re.sub(r"[^A-Za-z0-9._-]+", "_", Path(row["pdf"]).stem)
    metadata = curated.get(key)
    if metadata is None:
        return {"gate_verdict": "UNREVIEWED", "reference_only": True}
    if metadata.get("source_revision") != DATASET_REVISION or metadata.get("src_gt_sha256") != oracle_sha:
        raise ValueError("curated reference metadata differs from pinned raw source")
    if not isinstance(metadata.get("gate_verdict"), str):
        raise ValueError("curated reference metadata lacks a verdict")
    return metadata


def _artifact(root: Path, relative: str) -> dict:
    path = root / relative
    return {"file": relative, "bytes": path.stat().st_size, "sha256": sha256(path)}


def _materialize(row: dict, pin: AssetPin, cache: Path, output: Path, curated: dict) -> dict:
    record_id = pin.record_id
    pdf = f"inputs/{record_id}.pdf"
    oracle = f"oracles/{record_id}.html"
    shutil.copyfile(cache / pin.path, output / pdf)
    with (output / oracle).open("xb") as stream:
        stream.write(row["expected_markdown"].encode("utf-8"))
    pdf_record = _artifact(output, pdf)
    if pdf_record["sha256"] != pin.sha256 or pdf_record["bytes"] != pin.size:
        raise ValueError("copied PDF differs from upstream pin")
    oracle_record = _artifact(output, oracle)
    return {
        "id": record_id,
        "pdf": pdf_record,
        "oracle": oracle_record,
        "upstream": row,
        "curation": _curation(row, curated, oracle_record["sha256"]),
        "materialization": "verified_upstream_cache_copy",
    }


def build_stage(
    corpus_root: Path,
    cache_root: Path,
    output: Path,
    *,
    registry: Path | None = None,
    curated_manifest: Path | None = None,
) -> dict:
    """Verify the complete acquisition cache and create an external qualification stage."""
    helper = _helpers(corpus_root)
    cache = helper.safe_cache_root(cache_root)
    destination = helper.safe_cache_root(output)
    if destination.exists() and (not destination.is_dir() or any(destination.iterdir())):
        raise ValueError("stage output must be absent or empty")
    registry = registry or corpus_root / "scripts/data/benchmark-upstreams.json"
    registry_sha = sha256(registry)
    pins, pdfs = _verified_inputs(helper, registry, cache)
    jsonl = cache / next(pin.path for pin in pins if pin.kind == "jsonl")
    rows = _raw_rows(jsonl, pdfs)
    curated_sha = sha256(curated_manifest) if curated_manifest else None
    curated = _curated(curated_manifest)
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "inputs").mkdir()
    (destination / "oracles").mkdir()
    records = [_materialize(row, pdfs[row["pdf"]], cache, destination, curated) for row in rows]
    _verify_cache(helper, pins, cache)
    if sha256(registry) != registry_sha or (curated_manifest and sha256(curated_manifest) != curated_sha):
        raise ValueError("source metadata changed while staging")
    manifest = _manifest(helper, records, registry_sha, curated_sha)
    with (destination / "stage.json").open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n")
    return read_stage(destination)


def _manifest(helper: ModuleType, records: list[dict], registry_sha: str, curated_sha: str | None) -> dict:
    source = helper.SOURCES[SOURCE]
    return {
        "schema_version": 1,
        "dataset_revision": DATASET_REVISION,
        "evaluator_revision": EVALUATOR_REVISION,
        "dataset": source["dataset"],
        "evaluator": source["evaluator"],
        "annotations_license": source["annotations_license"],
        "annotations_license_url": source["annotations_license_url"],
        "attribution": source["attribution"],
        "pdf_redistribution": "reference-only; no redistribution authorized",
        "content_rights": source["content_rights"],
        "table_jsonl_sha256": JSONL_SHA256,
        "upstream_inventory_sha256": registry_sha,
        "curated_manifest_sha256": curated_sha,
        "records": sorted(records, key=lambda row: row["id"]),
    }


def main() -> None:
    """Build an offline stage from the separately acquired pinned upstream cache."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus-root", type=Path, required=True)
    parser.add_argument("--cache-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--registry", type=Path)
    parser.add_argument("--curated-manifest", type=Path)
    args = parser.parse_args()
    result = build_stage(
        args.corpus_root, args.cache_root, args.output, registry=args.registry, curated_manifest=args.curated_manifest
    )
    sys.stdout.write(
        json.dumps(
            {"records": len(result["records"]), "pdf_bytes": sum(row["pdf"]["bytes"] for row in result["records"])}
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
