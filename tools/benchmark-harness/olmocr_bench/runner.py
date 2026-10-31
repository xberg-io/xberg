"""Freeze, prepare, generate and score a complete official olmOCR comparison."""

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path

from .catalog import load_catalog
from .contracts import (
    ROLES,
    TEST_COUNT,
    digest,
    schedule,
    validate_argv,
    validate_config,
    validate_results,
    verify_assets,
    verify_predictions,
)
from .freeze import _dependencies, create_plan
from .official import evaluation_environment, load_official, score_candidate


def _write(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def _verify(plan: dict, *, corpus: bool = False) -> None:
    assets = [
        plan["config"],
        plan["ort"],
        plan["requirements"],
        plan["python"],
        plan["catalog"]["descriptor"],
        plan["renderer_manifest"],
        *plan["models"],
        *plan["evaluator_assets"],
        *plan["renderer_assets"],
    ]
    for producer in plan["producers"].values():
        assets.extend([producer["executable"], *producer["evidence"]])
    if corpus:
        assets.extend(plan["catalog"]["input_assets"])
    verify_assets(assets)
    validate_argv(plan["argv"])
    validate_config(json.loads(Path(plan["config"]["file"]).read_text()))
    if plan["cases"] != schedule(plan["catalog"]["pdfs"]) or len(plan["cases"]) != 1968:
        raise ValueError("generation case schedule changed")
    if len(plan["catalog"]["tests"]) != TEST_COUNT or plan["repeats"] != 1:
        raise ValueError("official test/repeat census changed")


def _read_plan(path: Path) -> dict:
    plan = json.loads(path.read_text())
    _verify(plan, corpus=True)
    registry = plan["catalog"]["registry"]
    catalog = load_catalog(Path(plan["data_root"]), Path(registry) if registry else None)
    if catalog != plan["catalog"] or _dependencies(Path(plan["requirements"]["file"])) != plan["dependencies"]:
        raise ValueError("catalog or installed environment differs from frozen plan")
    return plan


def _ready(plan_path: Path, ready_path: Path) -> None:
    ready = json.loads(ready_path.read_text())
    if ready["plan_sha256"] != digest(plan_path) or ready["initialized"] != TEST_COUNT or ready["errors"]:
        raise ValueError("all 6090 official tests must initialize before native generation")


def prepare(plan_path: Path, output: Path) -> None:
    """Fail before inference if any official reference/test cannot initialize."""
    plan = _read_plan(plan_path)
    output.mkdir(parents=True, exist_ok=False)
    with evaluation_environment(Path(plan["evaluator_root"]), Path(plan["browser_root"]), output / "home"):
        tests, rows = load_official(plan["catalog"]["tests"])
    _verify(plan, corpus=True)
    errors = [row for row in rows if row["status"] != "initialized"]
    _write(
        output / "ready.json",
        {
            "plan_sha256": digest(plan_path),
            "attempted": len(rows),
            "initialized": len(tests),
            "errors": errors,
            "rows": rows,
        },
    )
    if errors:
        raise ValueError("official test initialization failed; inspect retained error rows")
    (output / "ready.json").chmod(0o444)


def _model_fallback(document: dict, stderr: str) -> bool:
    warnings = document.get("processing_warnings", [])
    lines = [f"{row['source']} {row['message']}" for row in warnings] + stderr.splitlines()
    for line in lines:
        text = line.lower()
        model = any(word in text for word in ("layout", "formula", "latex", "tatr", "rt-detr", "model", "onnx"))
        failure = any(
            word in text
            for word in ("unavailable", "not available", "failed", "skipped", "fallback", "not compiled", "disabled")
        )
        if model and failure:
            return True
    return False


def _extract(plan: dict, case: dict, pdf: dict, diagnostics: Path, timeout: float) -> tuple[str, dict]:
    producer = plan["producers"][case["role"]]
    pdf_path = Path(plan["data_root"]) / pdf["file"]
    verify_assets([{"file": str(pdf_path), "sha256": pdf["sha256"]}])
    substitutions = {"pdf": str(pdf_path), "config": plan["config"]["file"]}
    argv = [producer["executable"]["file"], *(argument.format_map(substitutions) for argument in plan["argv"])]
    with tempfile.TemporaryDirectory(prefix="olmocr-result-cache-") as cache:
        environment = {
            "PATH": os.environ.get("PATH", ""),
            "HOME": cache,
            **plan["environment"],
            "XDG_CACHE_HOME": cache,
            "XBERG_CACHE_DIR": cache,
            "TMPDIR": cache,
        }
        try:
            completed = subprocess.run(argv, capture_output=True, timeout=timeout, env=environment, check=False)
        except subprocess.TimeoutExpired as error:
            (diagnostics / "stdout.json").write_bytes(error.stdout or b"")
            (diagnostics / "stderr.txt").write_bytes(error.stderr or b"")
            raise
    (diagnostics / "stdout.json").write_bytes(completed.stdout)
    (diagnostics / "stderr.txt").write_bytes(completed.stderr)
    if completed.returncode:
        raise ValueError(f"native extraction exited {completed.returncode}")
    envelope = json.loads(completed.stdout)
    document = envelope["result"]
    content = document["content"]
    if not isinstance(content, str):
        raise ValueError("native Markdown content is not a string")
    return content, {
        "processing_warnings": document.get("processing_warnings", []),
        "model_fallback": _model_fallback(document, completed.stderr.decode("utf-8", errors="replace")),
        "extraction_time_ms": envelope.get("extraction_time_ms"),
        "peak_memory_bytes": envelope.get("peak_memory_bytes"),
    }


def generate(plan_path: Path, ready: Path, output: Path, timeout: float) -> None:
    """Retain every real output and error; root explicitly dispatches native runs."""
    if timeout <= 0:
        raise ValueError("positive operator-selected extraction timeout required")
    plan = _read_plan(plan_path)
    _ready(plan_path, ready)
    output.mkdir(parents=True, exist_ok=False)
    pdfs = {row["id"]: row for row in plan["catalog"]["pdfs"]}
    rows = []
    for case in plan["cases"]:
        diagnostics = output / "diagnostics" / case["id"]
        diagnostics.mkdir(parents=True)
        pdf = pdfs[case["pdf_id"]]
        row = dict(case)
        try:
            _verify(plan)
            content, details = _extract(plan, case, pdf, diagnostics, timeout)
            _verify(plan)
            verify_assets([{"file": str(Path(plan["data_root"]) / pdf["file"]), "sha256": pdf["sha256"]}])
            relative = Path(pdf["file"].removeprefix("bench_data/pdfs/")).with_suffix("")
            prediction = output / case["role"] / f"{relative}_pg1_repeat1.md"
            prediction.parent.mkdir(parents=True, exist_ok=True)
            prediction.write_text(content, encoding="utf-8")
            row.update(
                details,
                prediction={"file": str(prediction.absolute()), "sha256": digest(prediction)},
                status="model_fallback"
                if details["model_fallback"]
                else "extracted"
                if content.strip()
                else "empty_prediction",
            )
        except Exception as error:
            row.update(status="generation_error", error=str(error))
        rows.append(row)
        with (output / "generation.jsonl").open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(row) + "\n")
    validate_results(plan["cases"], rows)
    _verify(plan, corpus=True)
    _write(
        output / "generation.json",
        {
            "plan_sha256": digest(plan_path),
            "expected": 1968,
            "rows": rows,
            "healthy": all(row["status"] == "extracted" for row in rows),
        },
    )
    if not all(row["status"] == "extracted" for row in rows):
        raise ValueError("generation completed with retained failures; scoring remains diagnostic")


def score(plan_path: Path, ready: Path, generation: Path, output: Path) -> None:
    """Run every official test for both producers, retaining errors and group rates."""
    plan = _read_plan(plan_path)
    _ready(plan_path, ready)
    generated = json.loads((generation / "generation.json").read_text())
    if generated["plan_sha256"] != digest(plan_path):
        raise ValueError("generation was produced under another plan")
    validate_results(plan["cases"], generated["rows"])
    predictions = [row["prediction"] for row in generated["rows"] if "prediction" in row]
    pdfs = {row["id"]: row for row in plan["catalog"]["pdfs"]}
    for row in generated["rows"]:
        if "prediction" in row:
            relative = Path(pdfs[row["pdf_id"]]["file"].removeprefix("bench_data/pdfs/")).with_suffix("")
            expected = generation / row["role"] / f"{relative}_pg1_repeat1.md"
            if Path(row["prediction"]["file"]).absolute() != expected.absolute():
                raise ValueError("prediction path does not match scheduled PDF/producer")
    verify_predictions(generation, predictions)
    output.mkdir(parents=True, exist_ok=False)
    reports = {}
    with evaluation_environment(Path(plan["evaluator_root"]), Path(plan["browser_root"]), output / "home"):
        tests, initialized = load_official(plan["catalog"]["tests"])
        _write(output / "initialization.json", {"rows": initialized})
        if len(tests) != TEST_COUNT:
            raise ValueError("official initialization failed; every attempted row retained")
        for role in ROLES:

            def verify() -> None:
                _verify(plan, corpus=True)
                verify_predictions(generation, predictions)

            reports[role] = score_candidate(generation / role, plan["catalog"]["tests"], tests, verify)
            _write(output / f"{role}.json", reports[role])
    _verify(plan, corpus=True)
    healthy = all(row.get("status") == "extracted" and row.get("prediction") for row in generated["rows"]) and not any(
        report["errors"] for report in reports.values()
    )
    _write(
        output / "coverage.json",
        {
            "plan_sha256": digest(plan_path),
            "generation_cases": len(generated["rows"]),
            "official_test_results": sum(len(report["tests"]) for report in reports.values()),
            "healthy": healthy,
        },
    )
    if not healthy:
        raise ValueError("qualification invalid: retained native/evaluator errors must be resolved")


def main() -> None:
    """Separate preparation and real native generation into explicit actions."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    for action in ("freeze", "prepare", "generate", "score"):
        command = subparsers.add_parser(action)
        command.add_argument("--plan", type=Path, required=True)
        if action == "freeze":
            command.add_argument("--data-root", type=Path, required=True)
            command.add_argument("--spec", type=Path, required=True)
        else:
            command.add_argument("--output", type=Path, required=True)
        if action in ("generate", "score"):
            command.add_argument("--prepared", type=Path, required=True)
        if action == "generate":
            command.add_argument("--timeout", type=float, required=True)
        if action == "score":
            command.add_argument("--generation", type=Path, required=True)
    args = parser.parse_args()
    if args.action == "freeze":
        create_plan(args.data_root, args.spec, args.plan)
    elif args.action == "prepare":
        prepare(args.plan, args.output)
    elif args.action == "generate":
        generate(args.plan, args.prepared, args.output, args.timeout)
    else:
        score(args.plan, args.prepared, args.generation, args.output)


if __name__ == "__main__":
    main()
