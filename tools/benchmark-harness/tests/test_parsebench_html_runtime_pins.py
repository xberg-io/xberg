"""Freeze real runner paths; synthetic producer bytes never establish extraction quality."""

import contextlib
import hashlib
import importlib.util
import json
import re
import shutil
import socket
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# ruff: noqa: PT027


class ProducerReachedError(RuntimeError):
    """Stop at the real subprocess boundary without running an extractor."""


def _asset(root: Path, name: str, content: bytes) -> dict:
    path = root / name
    path.write_bytes(content)
    return {"file": str(path), "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()}


@contextlib.contextmanager
def _copied_runner(root: Path):
    source = Path(__file__).parents[1] / "parsebench_html"
    package = root / "runtime"
    shutil.copytree(source, package, ignore=shutil.ignore_patterns("__pycache__"))
    names = ("contracts", "safeguards", "official")
    previous = {name: sys.modules.pop(name, None) for name in names}
    try:
        with patch.object(sys, "path", [str(package), *sys.path]):
            spec = importlib.util.spec_from_file_location("runtime_pin_runner", package / "runner.py")
            runner = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(runner)
            yield runner, package
    finally:
        for name, module in previous.items():
            sys.modules.pop(name, None)
            if module is not None:
                sys.modules[name] = module


def _stage(root: Path, runner) -> None:
    pdf = _asset(root, "source.pdf", b"%PDF-1.4\nsource integrity control")
    oracle = _asset(root, "oracle.html", b"<table><tr><td>Reference</td></tr></table>")
    for asset in (pdf, oracle):
        asset["file"] = Path(asset["file"]).name
    stage = {
        "dataset_revision": "2805a1d940f95a203e0ae4b88be9934f7765b3fc",
        "evaluator_revision": "328c71109e0b955f66583c33ec95ac52d9749db4",
        "records": [
            {
                "id": f"source-{index}",
                "pdf": pdf,
                "oracle": oracle,
                "upstream": {"rule": "{}"},
                "curation": {"gate_verdict": "ACCEPT"},
            }
            for index in range(503)
        ],
    }
    runner._write(root / "stage.json", stage)


def _configuration(arm: str) -> dict:
    config = {
        "output_format": "html",
        "use_cache": False,
        "concurrency": {"max_threads": 1},
        "ocr": {"tesseract_config": {"use_cache": False}},
        "force_ocr": arm == "forced_ocr_layout_html",
        "disable_ocr": arm == "native_html",
        "layout": None
        if arm == "native_html"
        else {
            "strategy": "always",
            "table_model": "tatr",
            "acceleration": {"provider": "cpu"},
        },
    }
    if arm == "layout_tatr_html":
        config["use_layout_for_markdown"] = True
    return config


def _producer(root: Path, runner) -> dict:
    executable = _asset(root, "producer", b"non-executed producer integrity control")
    ort = _asset(root, "ort", b"non-loaded runtime integrity control")
    pilot = _asset(root, "pilot", b"pilot integrity control, not model activation evidence")
    runner._write(
        root / "build.json",
        {
            "commit": "a" * 40,
            "executable_sha256": executable["sha256"],
            "features": ["pdf", "ocr", "layout-detection"],
        },
    )
    runner._write(
        root / "runtime.json",
        {
            "executable_sha256": executable["sha256"],
            "ort_sha256": ort["sha256"],
            "ort_version": "integrity-control",
            "execution_provider": "cpu",
            "positive_model_pilot": True,
            "pilot_artifacts": [pilot],
        },
    )
    arms = {}
    for arm in runner.ARMS:
        path = root / f"{arm}.json"
        runner._write(path, _configuration(arm))
        arms[arm] = {
            "config": str(path),
            "argv": ["extract", "{pdf}", "--config", "{config}", "--format", "json"],
            "content_path": ["result", "content"],
        }
    return {
        "commit": "a" * 40,
        "executable": executable["file"],
        "build_evidence": str(root / "build.json"),
        "runtime_evidence": str(root / "runtime.json"),
        "arms": arms,
        "environment": {
            "HF_HUB_CACHE": str(root),
            "TESSDATA_PREFIX": str(root),
            "ORT_DYLIB_PATH": ort["file"],
            "XBERG_ORT_EP": "cpu",
            "HF_HUB_OFFLINE": "1",
            "OMP_NUM_THREADS": "1",
            "RAYON_NUM_THREADS": "1",
        },
    }


def _freeze(root: Path, runner) -> tuple[Path, dict]:
    _stage(root, runner)
    producer = _producer(root, runner)
    model = _asset(root, "model", b"non-loaded model integrity control")
    lock = _asset(root, "requirements.lock", b"dependency integrity control")
    evaluator = Path(runner.upstream_parse.__file__).resolve().parents[4]
    specification = {
        "baseline": producer,
        "candidate": producer,
        "models": [model],
        "evaluator_root": str(evaluator),
        "requirements_lock": lock["file"],
    }
    runner._write(root / "spec.json", specification)
    destination = root / "frozen.json"
    runner.freeze(root, root / "spec.json", destination)
    plan = json.loads(destination.read_text())
    assert len(plan["cases"]) == len({case["id"] for case in plan["cases"]}) == 3018
    return destination, plan


@unittest.skipUnless(importlib.util.find_spec("parse_bench"), "requires the pinned offline evaluator environment")
class RuntimePinsTest(unittest.TestCase):
    """Exercise actual freeze/run preflight before the subprocess invocation boundary."""

    def _rejected(self, runner, root: Path, frozen: Path, pattern: str) -> None:
        error = None
        with patch.object(runner.subprocess, "run", side_effect=ProducerReachedError("producer reached")) as producer:
            try:
                runner.run(root, frozen, root / "results", 1)
            except (ValueError, ProducerReachedError) as caught:
                error = caught
            assert producer.call_count == 0, "unverified runtime reached the producer"
        assert isinstance(error, ValueError)
        assert re.search(pattern, str(error)), str(error)

    def test_actual_freeze_binds_complete_nonempty_runtime_inventory(self):
        """Freeze includes runner, imported guard and every own Python runtime source."""
        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.object(socket.socket, "connect", side_effect=AssertionError),
        ):
            root = Path(temporary)
            with _copied_runner(root) as (runner, package):
                _, plan = _freeze(root, runner)
                assert isinstance(plan.get("runtime_sources"), dict)
                pins = plan["runtime_sources"]
                assert pins["version"] == 1
                assert set(pins["files"]) == {str(p.relative_to(package)) for p in package.rglob("*.py")}
                assert len(pins["files"]) >= 5
                for name in ("runner.py", "safeguards.py", "contracts.py", "official.py"):
                    assert pins["files"][name] == runner.sha256(package / name)

    def test_unchanged_runtime_reaches_real_producer_boundary(self):
        """Valid unchanged pins must progress beyond preflight, not pass by doing zero work."""
        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.object(socket.socket, "connect", side_effect=AssertionError),
        ):
            root = Path(temporary)
            with _copied_runner(root) as (runner, _):
                frozen, _ = _freeze(root, runner)
                with patch.object(
                    runner.subprocess, "run", side_effect=ProducerReachedError("producer reached")
                ) as producer:
                    with self.assertRaisesRegex(ProducerReachedError, "producer reached"):
                        runner.run(root, frozen, root / "results", 1)
                    assert producer.call_count == 1

    def test_changed_runner_or_imported_guard_is_rejected_before_producer(self):
        """Actual loaded runner and guard byte changes cannot execute a frozen study."""
        for filename in ("runner.py", "safeguards.py"):
            with self.subTest(filename=filename), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                with _copied_runner(root) as (runner, package):
                    frozen, _ = _freeze(root, runner)
                    path = package / filename
                    path.write_text(path.read_text() + "\nCHANGED_RUNTIME = True\n")
                    self._rejected(runner, root, frozen, f"runtime source changed: {filename}")

    def test_missing_or_added_runtime_sources_are_rejected_before_producer(self):
        """Source inventory includes package initialization and entry points, not a fixed allowlist."""
        for filename in ("safeguards.py", "__init__.py", "__main__.py"):
            with self.subTest(filename=filename), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                with _copied_runner(root) as (runner, package):
                    frozen, _ = _freeze(root, runner)
                    path = package / filename
                    if path.exists():
                        path.unlink()
                    else:
                        path.write_text("PACKAGE_ENTRY = True\n")
                    self._rejected(runner, root, frozen, "runtime source inventory changed")

    def test_legacy_unbound_plan_requires_explicit_refreeze_before_run(self):
        """Old artifacts remain readable but cannot acquire source-protected execution silently."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with _copied_runner(root) as (runner, _):
                _, plan = _freeze(root, runner)
                plan.pop("runtime_sources", None)
                legacy = root / "legacy.json"
                runner._write(legacy, plan)
                assert json.loads(legacy.read_text())["cases"] == plan["cases"]
                self._rejected(runner, root, legacy, "runtime source pins.*refreeze")
