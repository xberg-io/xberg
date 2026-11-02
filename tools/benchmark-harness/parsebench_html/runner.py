"""Freeze and execute a complete offline ParseBench HTML comparison."""

import argparse
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from contracts import ARMS, ROLES, read_stage, schedule, sha256, validate_config, validate_rows, verified_file
from official import evaluate
from parse_bench.evaluation.evaluators import parse as upstream_parse
from safeguards import (
    metric_applicability,
    snapshot,
    validate_build,
    validate_diagnostics,
    validate_invocation,
    verify_bound_files,
)


def _write(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _freeze_environment(binary: dict, assets: Path, role: str) -> None:
    environment = binary["environment"]
    required = {
        "HF_HUB_CACHE",
        "TESSDATA_PREFIX",
        "ORT_DYLIB_PATH",
        "XBERG_ORT_EP",
        "HF_HUB_OFFLINE",
        "OMP_NUM_THREADS",
        "RAYON_NUM_THREADS",
    }
    optional = {"OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"}
    if not required <= environment.keys() or not environment.keys() <= required | optional:
        raise ValueError("explicit CPU/offline/model/tessdata environment required")
    if environment["XBERG_ORT_EP"] != "cpu" or environment["HF_HUB_OFFLINE"] != "1":
        raise ValueError("CPU execution and offline model cache required")
    for key in (required | optional) & environment.keys():
        if key.endswith("THREADS") and (not environment[key].isdigit() or int(environment[key]) < 1):
            raise ValueError("positive CPU thread pins required")
    for key in ("HF_HUB_CACHE", "TESSDATA_PREFIX"):
        environment[key] = str(Path(environment[key]).resolve(strict=True))
    binary["ort"] = snapshot(Path(environment["ORT_DYLIB_PATH"]), assets, f"{role}-ort")
    environment["ORT_DYLIB_PATH"] = binary["ort"]["file"]
    evidence_path = Path(binary["runtime_evidence"])
    evidence = json.loads(evidence_path.read_text())
    if evidence["executable_sha256"] != binary["sha256"] or evidence["ort_sha256"] != binary["ort"]["sha256"]:
        raise ValueError("runtime evidence does not match executable/ORT bytes")
    if (
        not evidence["ort_version"]
        or evidence["execution_provider"] != "cpu"
        or evidence["positive_model_pilot"] is not True
    ):
        raise ValueError("versioned CPU runtime and real positive model pilot required")
    binary["pilot_artifacts"] = []
    for artifact in evidence["pilot_artifacts"]:
        if sha256(Path(artifact["file"])) != artifact["sha256"]:
            raise ValueError("pilot artifact hash mismatch")
        binary["pilot_artifacts"].append(snapshot(Path(artifact["file"]), assets, f"{role}-pilot"))
    if not binary["pilot_artifacts"]:
        raise ValueError("positive pilot artifacts required")
    binary["runtime_evidence"] = snapshot(evidence_path, assets, f"{role}-runtime-evidence")


def _freeze_binary(binary: dict, assets: Path, role: str) -> None:
    if len(binary["commit"]) != 40 or any(c not in "0123456789abcdef" for c in binary["commit"]):
        raise ValueError("full git commit required")
    executable = snapshot(Path(binary["executable"]), assets, f"{role}-executable", executable=True)
    binary.update(executable=executable["file"], sha256=executable["sha256"])
    build = Path(binary["build_evidence"])
    validate_build(build, binary["commit"], binary["sha256"])
    binary["build_evidence"] = snapshot(build, assets, f"{role}-build-evidence")
    _freeze_environment(binary, assets, role)
    if set(binary["arms"]) != set(ARMS):
        raise ValueError("all three arms required")
    for arm, invocation in binary["arms"].items():
        validate_invocation(invocation)
        config = Path(invocation["config"])
        validate_config(json.loads(config.read_text()), arm)
        frozen = snapshot(config, assets, f"{role}-{arm}-config")
        invocation.update(config=frozen["file"], config_sha256=frozen["sha256"])


def freeze(root: Path, specification: Path, destination: Path) -> None:
    """Bind every invocation to immutable inputs before native extraction."""
    stage = read_stage(root)
    spec = json.loads(specification.read_text(encoding="utf-8"))
    if destination.exists():
        raise ValueError("refuse to replace frozen plan")
    assets = destination.with_suffix(".assets")
    for role in ROLES:
        _freeze_binary(spec[role], assets, role)
    for model in spec["models"]:
        path = Path(model["file"]).absolute()
        model.update(file=str(path), sha256=sha256(path), bytes=path.stat().st_size)
    if not spec["models"]:
        raise ValueError("layout/model provenance required")
    evaluator = Path(spec["evaluator_root"]).resolve(strict=True)
    git = shutil.which("git")
    if git is None:
        raise ValueError("git is required to verify evaluator provenance")
    revision = subprocess.check_output([git, "-C", str(evaluator), "rev-parse", "HEAD"], text=True).strip()
    if revision != stage["evaluator_revision"]:
        raise ValueError("evaluator revision mismatch")
    if subprocess.check_output([git, "-C", str(evaluator), "status", "--porcelain"], text=True).strip():
        raise ValueError("evaluator checkout must be clean")
    evaluator_files = {
        str(path.relative_to(evaluator)): sha256(path) for path in sorted((evaluator / "src").rglob("*.py"))
    }
    lock = Path(spec["requirements_lock"]).resolve(strict=True)
    spec.update(
        evaluator_root=str(evaluator),
        evaluator_files=evaluator_files,
        requirements_lock=str(lock),
        requirements_sha256=sha256(lock),
    )
    spec["stage_sha256"] = sha256(root / "stage.json")
    spec["cases"] = schedule(stage["records"], list(ROLES), list(ARMS))
    if len(spec["cases"]) != 3018:
        raise ValueError("expected exactly 3018 cases")
    _write(destination, spec)
    destination.chmod(0o444)


def _verify_plan(root: Path, plan: dict) -> dict:
    if not Path(upstream_parse.__file__).resolve().is_relative_to(Path(plan["evaluator_root"])):
        raise ValueError("imported evaluator does not match frozen checkout")
    if sha256(root / "stage.json") != plan["stage_sha256"]:
        raise ValueError("stage changed after freeze")
    stage = read_stage(root)
    if plan["cases"] != schedule(stage["records"], list(ROLES), list(ARMS)):
        raise ValueError("case schedule changed")
    for role in ROLES:
        binary = plan[role]
        _verify_producer(binary)
        for evidence in (binary["build_evidence"], binary["runtime_evidence"], *binary["pilot_artifacts"]):
            if sha256(Path(evidence["file"])) != evidence["sha256"]:
                raise ValueError("producer provenance changed")
    for model in plan["models"]:
        if sha256(Path(model["file"])) != model["sha256"]:
            raise ValueError("model changed")
    if sha256(Path(plan["requirements_lock"])) != plan["requirements_sha256"]:
        raise ValueError("dependency lock changed")
    for name, digest in plan["evaluator_files"].items():
        if sha256(Path(plan["evaluator_root"]) / name) != digest:
            raise ValueError("evaluator source changed")
    return stage


def _verify_producer(binary: dict) -> None:
    if sha256(Path(binary["executable"])) != binary["sha256"]:
        raise ValueError("executable changed")
    if sha256(Path(binary["ort"]["file"])) != binary["ort"]["sha256"]:
        raise ValueError("ORT library changed")
    for arm, invocation in binary["arms"].items():
        validate_invocation(invocation)
        config = Path(invocation["config"])
        if sha256(config) != invocation["config_sha256"]:
            raise ValueError("configuration changed")
        validate_config(json.loads(config.read_text()), arm)


def _verify_case_assets(root: Path, record: dict, plan: dict) -> None:
    verified_file(root, record["pdf"])
    verified_file(root, record["oracle"])
    assets = [*plan["models"], {"file": plan["requirements_lock"], "sha256": plan["requirements_sha256"]}]
    assets.extend(
        {"file": str(Path(plan["evaluator_root"]) / name), "sha256": digest}
        for name, digest in plan["evaluator_files"].items()
    )
    verify_bound_files(assets)


def _invoke(root: Path, record: dict, binary: dict, arm: str, output: Path, *, timeout: float) -> str:
    _verify_producer(binary)
    invocation = binary["arms"][arm]
    with tempfile.TemporaryDirectory(prefix="parsebench-cache-") as temporary:
        replacements = {
            "pdf": str(verified_file(root, record["pdf"])),
            "config": invocation["config"],
            "cache": temporary,
        }
        argv = [binary["executable"], *(argument.format_map(replacements) for argument in invocation["argv"])]
        environment = {key: os.environ[key] for key in ("HOME", "PATH", "LANG", "LC_ALL") if key in os.environ}
        environment.update(binary["environment"])
        environment.update(XDG_CACHE_HOME=temporary, XBERG_CACHE_DIR=temporary, TMPDIR=temporary)
        environment["LLAMACLOUD_BENCH_LLM_NORMALIZATION"] = "off"
        try:
            completed = subprocess.run(argv, capture_output=True, timeout=timeout, env=environment, check=False)
        finally:
            _verify_producer(binary)
    (output / "stdout.json").write_bytes(completed.stdout)
    (output / "stderr.txt").write_bytes(completed.stderr)
    if completed.returncode:
        raise ValueError(f"extraction exited {completed.returncode}")
    content = json.loads(completed.stdout)
    extracted = content
    for key in invocation["content_path"][:-1]:
        extracted = extracted[key]
    validate_diagnostics(extracted, completed.stderr.decode("utf-8", errors="replace"), arm)
    for key in invocation["content_path"]:
        content = content[key]
    if not isinstance(content, str):
        raise ValueError("prediction content is not a string")
    (output / "prediction.html").write_text(content, encoding="utf-8")
    return content


def run(root: Path, plan_path: Path, output: Path, timeout: float) -> None:
    """Count extraction and evaluation failures instead of dropping cases."""
    plan = json.loads(plan_path.read_text())
    stage = _verify_plan(root, plan)
    output.mkdir(parents=True, exist_ok=False)
    _write(output / "frozen-plan.json", plan)
    records = {record["id"]: record for record in stage["records"]}
    rows = []
    for case in plan["cases"]:
        destination = output / case["id"]
        destination.mkdir(parents=True)
        row = dict(case)
        row["curation_verdict"] = records[case["record_id"]]["curation"]["gate_verdict"]
        options = json.loads(records[case["record_id"]]["upstream"]["rule"])
        row["trm_supported"] = not options.get("trm_unsupported", False)
        try:
            _verify_case_assets(root, records[case["record_id"]], plan)
            prediction = _invoke(
                root, records[case["record_id"]], plan[case["role"]], case["arm"], destination, timeout=timeout
            )
            _verify_case_assets(root, records[case["record_id"]], plan)
            row["prediction"] = {
                "file": str((destination / "prediction.html").relative_to(output)),
                "sha256": sha256(destination / "prediction.html"),
            }
            row["status"] = "predicted" if prediction.strip() else "empty_prediction"
        except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
            row.update(status="extraction_error", error=str(error))
        else:
            try:
                result = evaluate(records[case["record_id"]], root, prediction)
                _verify_case_assets(root, records[case["record_id"]], plan)
                _write(destination / "evaluation.json", result)
                row["metrics"] = {item["metric_name"]: item["value"] for item in result["metrics"]}
                row["metric_applicability"] = metric_applicability(result, options)
                row["official_success"] = result["success"]
                row["evaluation_status"] = "evaluated"
            except Exception as error:
                row.update(evaluation_status="evaluation_error", error=str(error))
        rows.append(row)
        with (output / "results.jsonl").open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(row) + "\n")
    census = validate_rows(plan["cases"], rows)
    census["evaluated"] = sum(row.get("evaluation_status") == "evaluated" for row in rows)
    census["eligible_metric_cases"] = {
        name: sum(row.get("metric_applicability", {}).get(name) is True for row in rows)
        for name in ("grits_con", "table_record_match")
    }
    census["trm_unsupported_cases"] = sum(not row["trm_supported"] for row in rows)
    census["curation_verdicts"] = {
        verdict: sum(record["curation"]["gate_verdict"] == verdict for record in records.values())
        for verdict in {record["curation"]["gate_verdict"] for record in records.values()}
    }
    try:
        _verify_plan(root, plan)
    except (OSError, ValueError) as error:
        census["final_integrity"] = {"success": False, "error": str(error)}
    else:
        census["final_integrity"] = {"success": True}
    _write(output / "coverage.json", census)
    if census["evaluated"] != 3018 or not census["final_integrity"]["success"]:
        raise ValueError("incomplete qualification; inspect explicit case failures")


def main() -> None:
    """Require an explicit freeze or native-run action."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["freeze", "run"])
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True, help="Specification for freeze; frozen plan for run")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=300)
    args = parser.parse_args()
    if args.action == "freeze":
        freeze(args.stage, args.input, args.output)
    else:
        run(args.stage, args.input, args.output, args.timeout)


if __name__ == "__main__":
    main()
