"""Portable controls for the clean Windows wheel qualification gate."""

import csv
import hashlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

import windows_wheel_clean_host as clean
from resolve_windows_wheel_artifact import select_prior_artifact
from test_windows_wheel_closure import expected_error, pe, require_pwsh

ROOT = Path(__file__).resolve().parents[2]


class CleanHostGuards(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.wheel = self.root / "xberg-1-cp311-abi3-win_amd64.whl"
        self.files = {
            "xberg/_xberg.pyd": bytes(pe(["onnxruntime.dll"])),
            "xberg/onnxruntime.dll": bytes(pe(["msvcp140.dll"])),
            "xberg/msvcp140.dll": bytes(pe()),
            "xberg-1.dist-info/METADATA": b"Metadata-Version: 2.1\nName: xberg\nVersion: 1\n",
        }
        self.notice = {
            "component": "Microsoft Visual C++ Runtime",
            "distribution_authority": clean.closure.REDIST_TERMS,
            "runtime_use_terms": clean.closure.RUNTIME_TERMS,
            "files": [
                {"file": "xberg/msvcp140.dll", "sha256": hashlib.sha256(self.files["xberg/msvcp140.dll"]).hexdigest()}
            ],
        }

    def write_wheel(self) -> None:
        self.files["xberg-1.dist-info/licenses/MICROSOFT_VC_RUNTIME.json"] = json.dumps(self.notice).encode()
        stream = io.StringIO()
        writer = csv.writer(stream)
        for name, data in self.files.items():
            writer.writerow([name, clean.record_hash(data), len(data)])
        self.files["xberg-1.dist-info/RECORD"] = stream.getvalue().encode()
        with zipfile.ZipFile(self.wheel, "w") as archive:
            for name, data in self.files.items():
                archive.writestr(name, data)

    def test_intact_wheel_has_real_graph_and_complete_notices(self) -> None:
        self.write_wheel()
        result = clean.verify_wheel(self.wheel)
        assert result["closure"] == {"files": 3, "edges": 2, "roots": 1}
        assert result["msvcp"] == ["xberg/msvcp140.dll"]

    def test_missing_notice_file_is_rejected(self) -> None:
        self.notice["files"] = []
        self.write_wheel()
        with expected_error(ValueError, "notice inventory"):
            clean.verify_wheel(self.wheel)

    def test_notice_hash_mismatch_is_rejected(self) -> None:
        self.notice["files"][0]["sha256"] = "0" * 64
        self.write_wheel()
        with expected_error(ValueError, "notice hash"):
            clean.verify_wheel(self.wheel)

    def test_modified_recorded_bytes_are_rejected(self) -> None:
        self.write_wheel()
        with zipfile.ZipFile(self.wheel, "a") as archive:
            archive.writestr("extra.dll", pe())
        with expected_error(ValueError, "RECORD inventory"):
            clean.verify_wheel(self.wheel)

    def baseline_with_files(self, files: list[str], modules: list[Path] | None = None) -> dict:
        for name in files:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"inbox")
        registry = SimpleNamespace(
            KEY_WOW64_64KEY=1,
            KEY_WOW64_32KEY=2,
            HKEY_LOCAL_MACHINE=3,
            KEY_READ=4,
            OpenKey=Mock(side_effect=FileNotFoundError),
        )
        missing = OSError("missing")
        missing.winerror = 126
        with (
            patch.dict(sys.modules, {"winreg": registry}),
            patch.dict(clean.os.environ, {"SYSTEMROOT": str(self.root / "Windows")}),
            patch.object(clean.ctypes, "WinDLL", create=True, side_effect=missing) as load,
            patch.object(clean, "loaded_modules", return_value=modules or []),
        ):
            result = clean.baseline(self.root / "python", ["xberg/msvcp140.dll"])
        assert registry.OpenKey.call_count == 6
        load.assert_called_once_with("msvcp140.dll")
        return result

    def test_exact_inbox_clr_sidecar_is_allowed_in_system_directories(self) -> None:
        result = self.baseline_with_files(
            [
                "Windows/System32/msvcp140_clr0400.dll",
                "Windows/SysWOW64/MSVCP140_CLR0400.dll",
            ]
        )
        assert result["global_files_absent"] is True
        assert result["msvcp_load_failures"] == ["msvcp140.dll"]
        assert result["inbox_clr_files"] == [
            "SysWOW64/MSVCP140_CLR0400.dll",
            "System32/msvcp140_clr0400.dll",
        ]

    def test_inbox_allowance_does_not_cover_other_runtime_files_or_paths(self) -> None:
        for name in (
            "Windows/System32/msvcp140.dll",
            "Windows/SysWOW64/vcruntime140.dll",
            "Windows/System32/msvcp140_clr0401.dll",
            "Windows/System32/subdir/msvcp140_clr0400.dll",
            "python/msvcp140_clr0400.dll",
        ):
            with self.subTest(name=name):
                try:
                    with expected_error(ValueError, "baseline contains VC runtime"):
                        self.baseline_with_files([name])
                finally:
                    (self.root / name).unlink()

    def test_inbox_allowance_does_not_cover_loaded_modules(self) -> None:
        name = "Windows/System32/msvcp140_clr0400.dll"
        with expected_error(ValueError, "outside wheel"):
            self.baseline_with_files([name], [self.root / name])

    def test_global_or_python_msvcp_module_is_rejected(self) -> None:
        for location in (self.root / "Windows/System32", self.root / "python"):
            with self.subTest(location=location), expected_error(ValueError, "outside wheel"):
                clean.verify_modules([location / "msvcp140.dll"], self.root / "venv", self.root / "python")

    def test_python_app_local_vcruntime_is_allowed_but_not_global(self) -> None:
        clean.verify_modules([self.root / "python/vcruntime140.dll"], self.root / "venv", self.root / "python")
        with expected_error(ValueError, "outside allowed"):
            clean.verify_modules([self.root / "Windows/vcruntime140.dll"], self.root / "venv", self.root / "python")

    def test_successful_negative_probe_is_rejected(self) -> None:
        outcome = subprocess.run(
            [sys.executable, "-c", "print('unexpected success')"], capture_output=True, text=True, check=False
        )
        with expected_error(ValueError, "unexpectedly succeeded"):
            clean.require_missing_dependency(outcome, "msvcp140.dll")

    def test_wrong_reason_failure_is_rejected(self) -> None:
        outcome = subprocess.run(
            [sys.executable, "-c", "raise RuntimeError('unrelated')"], capture_output=True, text=True, check=False
        )
        with expected_error(ValueError, "not the required missing dependency"):
            clean.require_missing_dependency(outcome, "msvcp140.dll")

    def test_qualified_loader_failure_is_accepted(self) -> None:
        outcome = subprocess.run(
            [sys.executable, "-c", "import sys; print('MISSING_DEPENDENCY:msvcp140.dll:126'); sys.exit(23)"],
            capture_output=True,
            text=True,
            check=False,
        )
        clean.require_missing_dependency(outcome, "msvcp140.dll")

    def test_wrong_source_fails_before_docker_or_output_creation(self) -> None:
        output = self.root / "receipts"
        outcome = subprocess.run(
            [
                require_pwsh(),
                "-NoProfile",
                "-File",
                str(ROOT / "scripts/ci/qualify-windows-wheel-clean-host.ps1"),
                "-WheelDirectory",
                str(self.root),
                "-SourceSha",
                "0" * 40,
                "-WheelSourceSha",
                "1" * 40,
                "-OutputDirectory",
                str(output),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        assert outcome.returncode != 0
        assert "Checkout is not the exact requested source SHA" in outcome.stderr
        assert not output.exists()

    def test_empty_wheel_set_fails_before_output_or_docker(self) -> None:
        self.require_wheel_count_failure()

    def test_multiple_wheels_fail_before_output_or_docker(self) -> None:
        self.write_wheel()
        (self.root / "other-win_amd64.whl").write_bytes(self.wheel.read_bytes())
        self.require_wheel_count_failure()

    def require_wheel_count_failure(self) -> None:
        sha = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
        ).stdout.strip()
        output = self.root / "receipts"
        outcome = subprocess.run(
            [
                require_pwsh(),
                "-NoProfile",
                "-File",
                str(ROOT / "scripts/ci/qualify-windows-wheel-clean-host.ps1"),
                "-WheelDirectory",
                str(self.root),
                "-SourceSha",
                sha,
                "-OutputDirectory",
                str(output),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        assert outcome.returncode != 0
        assert "Expected exactly one Windows x64 wheel" in outcome.stderr
        assert not output.exists()

    def test_prior_artifact_requires_exact_producer_and_successful_upload(self) -> None:
        source = "a" * 40
        identity = {"id": 10, "head_branch": "main", "head_sha": source}
        run = {
            **identity,
            "event": "workflow_dispatch",
            "status": "completed",
            "path": ".github/workflows/ci-python-wheel-smoke.yaml",
            "repository": {"id": 42, "full_name": "xberg-io/xberg"},
            "head_repository": {"id": 42, "full_name": "xberg-io/xberg"},
        }
        upload = {
            "name": "Upload Windows wheel for isolated qualification",
            "conclusion": "success",
            "started_at": "2026-11-05T21:12:34Z",
            "completed_at": "2026-11-05T21:12:37Z",
        }
        job = {"name": "Build + smoke-test Python wheel (windows-2022)", "conclusion": "success", "steps": [upload]}
        artifact = {
            "id": 20,
            "name": f"clean-host-windows-wheel-{source}",
            "digest": "sha256:" + "b" * 64,
            "expired": False,
            "created_at": upload["completed_at"],
            "workflow_run": {**identity, "repository_id": 42, "head_repository_id": 42},
        }
        assert select_prior_artifact("xberg-io/xberg", 10, run, [job], [artifact]) == artifact
        for wrong_run in (
            {"head_branch": "other"},
            {"head_repository": {"id": 43, "full_name": "fork/xberg"}},
            {"path": ".github/workflows/other.yaml"},
            {"status": "in_progress"},
        ):
            with self.subTest(wrong_run=wrong_run), expected_error(ValueError, "producer run"):
                select_prior_artifact("xberg-io/xberg", 10, {**run, **wrong_run}, [job], [artifact])
        with expected_error(ValueError, "successful Windows upload"):
            select_prior_artifact("xberg-io/xberg", 10, run, [{**job, "conclusion": "failure"}], [artifact])
        for wrong_artifact in (
            {"expired": True},
            {"digest": None},
            {"name": "unrelated"},
            {"workflow_run": {**artifact["workflow_run"], "head_sha": "c" * 40}},
        ):
            with self.subTest(wrong_artifact=wrong_artifact), expected_error(ValueError, "artifact provenance"):
                select_prior_artifact("xberg-io/xberg", 10, run, [job], [{**artifact, **wrong_artifact}])
        with expected_error(ValueError, "artifact provenance"):
            select_prior_artifact("xberg-io/xberg", 10, run, [job], [artifact, artifact])
        with expected_error(ValueError, "successful Windows upload"):
            select_prior_artifact(
                "xberg-io/xberg", 10, run, [job], [{**artifact, "created_at": "2026-11-05T21:12:33Z"}]
            )

    def test_workflow_binds_main_and_collects_two_real_cases(self) -> None:
        workflow = (ROOT / ".github/workflows/ci-python-wheel-smoke.yaml").read_text()
        assert "windows-wheel-clean-host:" in workflow
        assert 'mise run python:wheel:clean-host wheelhouse "${{ github.sha }}"' in workflow
        assert 'install-args: "python"' in workflow
        driver = (ROOT / "scripts/ci/qualify-windows-wheel-clean-host.ps1").read_text()
        assert "--isolation=process" in driver
        assert "foreach ($case in @('intact', 'missing-runtime'))" in driver
        assert "--isolation=hyperv" not in driver
        assert "C:\\Windows:" not in driver

    def test_reuse_skips_build_and_verifies_distinct_producer_and_checkout(self) -> None:
        workflow = (ROOT / ".github/workflows/ci-python-wheel-smoke.yaml").read_text()
        assert "inputs.windows_producer_run_id == ''" in workflow.split("windows-wheel-clean-host:", 1)[0]
        assert "needs.python-wheel-smoke.result == 'skipped'" in workflow
        assert "artifact-ids: ${{ steps.prior-wheel.outputs.artifact_id }}" in workflow
        assert "run-id: ${{ inputs.windows_producer_run_id }}" in workflow
        assert "digest-mismatch: error" in workflow
        assert '-SourceSha "$env:GITHUB_SHA" -WheelSourceSha "$env:WHEEL_SOURCE_SHA"' in workflow
        driver = (ROOT / "scripts/ci/qualify-windows-wheel-clean-host.ps1").read_text()
        assert "$actual -ne $SourceSha" in driver
        assert "source_sha = $WheelSourceSha" in driver
        assert "verifier_sha = $actual" in driver

    def test_dispatch_can_select_windows_without_changing_the_full_default(self) -> None:
        workflow = (ROOT / ".github/workflows/ci-python-wheel-smoke.yaml").read_text()
        assert "default: all" in workflow
        assert "inputs.platform == 'windows'" in workflow
        assert "'[\"windows-2022\"]'" in workflow
        assert '\'["ubuntu-latest","ubuntu-24.04-arm","windows-2022","macos-26","macos-26-intel"]\'' in workflow


if __name__ == "__main__":
    unittest.main()
