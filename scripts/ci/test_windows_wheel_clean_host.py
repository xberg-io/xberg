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

import windows_wheel_clean_host as clean
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

    def test_dispatch_can_select_windows_without_changing_the_full_default(self) -> None:
        workflow = (ROOT / ".github/workflows/ci-python-wheel-smoke.yaml").read_text()
        assert "default: all" in workflow
        assert "inputs.platform == 'windows'" in workflow
        assert "'[\"windows-2022\"]'" in workflow
        assert '\'["ubuntu-latest","ubuntu-24.04-arm","windows-2022","macos-26","macos-26-intel"]\'' in workflow


if __name__ == "__main__":
    unittest.main()
