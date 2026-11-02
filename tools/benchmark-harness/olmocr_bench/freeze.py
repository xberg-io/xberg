"""Freeze one identical configuration and the real producer/runtime identities."""

import importlib.metadata
import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path

from .catalog import file_binding, load_catalog
from .contracts import (
    EVALUATOR_REVISION,
    ROLES,
    schedule,
    snapshot,
    validate_argv,
    validate_config,
    verify_assets,
)


def _dependencies(lock: Path) -> dict:
    versions = dict(re.findall(r"^([A-Za-z0-9_.-]+)==([^\s\\]+)", lock.read_text(), flags=re.MULTILINE))
    if not versions:
        raise ValueError("empty dependency lock")
    for name, version in versions.items():
        if importlib.metadata.version(name) != version:
            raise ValueError(f"installed dependency differs from audited lock: {name}")
    return versions


def _environment(spec: dict, assets: Path) -> tuple[dict, dict]:
    env = spec["environment"]
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
    if not required <= env.keys() or not env.keys() <= required | optional:
        raise ValueError("explicit CPU/offline/model/tessdata environment required")
    if env["XBERG_ORT_EP"] != "cpu" or env["HF_HUB_OFFLINE"] != "1":
        raise ValueError("CPU and offline model discovery required")
    for name, value in env.items():
        if name.endswith("THREADS") and (not value.isdigit() or int(value) < 1):
            raise ValueError("positive thread pins required")
    for name in ("HF_HUB_CACHE", "TESSDATA_PREFIX"):
        env[name] = str(Path(env[name]).resolve(strict=True))
    ort = snapshot(Path(env["ORT_DYLIB_PATH"]), assets, "ort")
    env["ORT_DYLIB_PATH"] = ort["file"]
    return env, ort


def _producer(binary: dict, role: str, config: dict, ort: dict, models: list[dict], *, assets: Path) -> dict:
    if not re.fullmatch(r"[a-f0-9]{40}", binary["commit"]):
        raise ValueError("full producer commit required")
    executable = snapshot(Path(binary["executable"]), assets, role, executable=True)
    build_path = Path(binary["build_evidence"])
    build = json.loads(build_path.read_text())
    if build["commit"] != binary["commit"] or build["executable_sha256"] != executable["sha256"]:
        raise ValueError("build evidence does not match executable")
    if not {"pdf", "ocr", "layout-detection", "formula-recognition"} <= set(build["features"]):
        raise ValueError("required PDF/OCR/layout/formula features missing")
    runtime_path = Path(binary["runtime_evidence"])
    runtime = json.loads(runtime_path.read_text())
    for key, expected in (
        ("executable_sha256", executable["sha256"]),
        ("config_sha256", config["sha256"]),
        ("ort_sha256", ort["sha256"]),
        ("execution_provider", "cpu"),
        ("positive_model_pilot", True),
    ):
        if runtime.get(key) != expected:
            raise ValueError(f"runtime evidence mismatch: {key}")
    if not runtime["ort_version"] or set(runtime["model_sha256"]) != {row["sha256"] for row in models}:
        raise ValueError("versioned runtime and exact model pilot identities required")
    verify_assets(runtime["pilot_artifacts"])
    pilots = [snapshot(Path(row["file"]), assets, f"{role}-pilot") for row in runtime["pilot_artifacts"]]
    return {
        "commit": binary["commit"],
        "executable": executable,
        "evidence": [
            snapshot(build_path, assets, f"{role}-build"),
            snapshot(runtime_path, assets, f"{role}-runtime"),
            *pilots,
        ],
    }


def create_plan(root: Path, specification: Path, destination: Path) -> None:
    """Write an immutable reviewed plan; perform no document extraction."""
    root = root.resolve(strict=True)
    destination = destination.absolute()
    if destination.exists():
        raise ValueError("refuse to replace frozen plan")
    spec = json.loads(specification.read_text())
    catalog = load_catalog(root, Path(spec["registry"]).absolute() if spec.get("registry") else None)
    validate_argv(spec["argv"])
    config_path = Path(spec["config"])
    validate_config(json.loads(config_path.read_text()))
    assets = destination.with_suffix(".assets")
    config = snapshot(config_path, assets, "config")
    environment, ort = _environment(spec, assets)
    models = [dict(row, **file_binding(Path(row["file"]))) for row in spec["models"]]
    if not models or any(not row.get("revision") or not row.get("license") for row in models):
        raise ValueError("actual model revisions and licenses required")
    evaluator = Path(spec["evaluator_root"]).resolve(strict=True)
    git = shutil.which("git")
    if git is None:
        raise ValueError("git required for evaluator provenance")
    if (
        subprocess.check_output([git, "-C", str(evaluator), "rev-parse", "HEAD"], text=True).strip()
        != EVALUATOR_REVISION
    ):
        raise ValueError("official evaluator revision mismatch")
    if subprocess.check_output([git, "-C", str(evaluator), "status", "--porcelain"], text=True).strip():
        raise ValueError("official evaluator checkout is dirty")
    sources = [file_binding(path) for path in sorted((evaluator / "olmocr").rglob("*.py"))]
    sources.extend(file_binding(path) for path in sorted((evaluator / "olmocr/bench/katex").glob("*.js")))
    sources.extend(file_binding(path) for path in sorted((evaluator / "olmocr/bench/katex").glob("*.css")))
    renderer_path = Path(spec["renderer_manifest"]).resolve(strict=True)
    renderer = json.loads(renderer_path.read_text())
    renderer_root = renderer_path.parent
    rendered_assets = [
        {"file": str(renderer_root / row["file"]), "sha256": row["sha256"]}
        for row in renderer["downloaded_files"] + renderer["katex_assets"] + [renderer["browser_manifest"]]
    ]
    verify_assets(rendered_assets)
    if importlib.metadata.version("playwright") != renderer["playwright_version"]:
        raise ValueError("renderer/Playwright versions differ")
    lock_path = Path(spec["requirements_lock"])
    versions = _dependencies(lock_path)
    lock = snapshot(lock_path, assets, "requirements")
    producers = {role: _producer(spec[role], role, config, ort, models, assets=assets) for role in ROLES}
    plan = {
        "schema_version": 1,
        "data_root": str(root.resolve()),
        "catalog": catalog,
        "config": config,
        "ort": ort,
        "environment": environment,
        "models": models,
        "producers": producers,
        "argv": spec["argv"],
        "evaluator_root": str(evaluator),
        "evaluator_assets": sources,
        "renderer_assets": rendered_assets,
        "renderer_manifest": file_binding(renderer_path),
        "browser_root": str(renderer_root / "browsers"),
        "requirements": lock,
        "dependencies": versions,
        "python": file_binding(Path(os.sys.executable)),
        "hardware": {"platform": platform.platform(), "machine": platform.machine(), "cpu_count": os.cpu_count()},
        "cases": schedule(catalog["pdfs"]),
        "expected_generation_cases": 1968,
        "expected_test_results": 12180,
        "repeats": 1,
    }
    destination.write_text(json.dumps(plan, indent=2) + "\n")
    destination.chmod(0o444)
