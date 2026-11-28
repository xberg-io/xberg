import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import shutil
import zipfile
import re
import unittest

from setup_checks import ROOT, version_sources

SCRIPTS = ROOT / "scripts/ci/actions/setup-onnx-runtime"


def _bash() -> str:
    executable = shutil.which("bash")
    if executable is None:
        raise RuntimeError("bash is required for native setup checks")
    return executable


class SetupContract(unittest.TestCase):
    """Exercise real setup files and native archive refusal boundaries."""

    def test_shared_version_sources_match_and_detect_drift(self):
        """Keep shared versions and the NLI verification opt-in bound together."""
        action = (ROOT / ".github/actions/setup-onnx-runtime/action.yml").read_text()
        workflow = (ROOT / ".github/workflows/ci-rust.yaml").read_text()
        version_sources(action, workflow)
        missing_consumer = workflow.replace("uses: ./.github/actions/setup-onnx-runtime", "uses: ./unused-action", 1)
        with self.assertRaisesRegex(ValueError, "expected exactly six main ORT action consumers, found 5"):  # noqa: PT027 -- Setup runs with stdlib only.
            version_sources(action, missing_consumer)
        focused = (ROOT / ".github/workflows/ci-focused-pr.yaml").read_text()
        marker = "      - name: Setup ONNX Runtime for NLI admission tests"
        assert focused.count(marker) == 1
        step = focused.split(marker)[1].split("      - name:", 1)[0]
        assert 'verify-linux-archive: "true"' in step
        assert "ort-version:" not in step
        for bad_action, bad_workflow in [
            (action, workflow + '\n  ORT_VERSION: "different"\n'),
            ("", workflow),
        ]:
            with self.assertRaisesRegex(ValueError, "single native action default"):  # noqa: PT027 -- Setup runs with stdlib only.
                version_sources(bad_action, bad_workflow)

    def test_each_nli_linux_runtime_has_one_package_and_one_wheel_digest(self):
        """Require one NuGet package pin and one PyPI wheel pin for each supported Linux architecture."""
        action = (ROOT / ".github/actions/setup-onnx-runtime/action.yml").read_text()
        workflow = (ROOT / ".github/workflows/ci-rust.yaml").read_text()
        version = version_sources(action, workflow)
        lines = [
            line.split()
            for line in (SCRIPTS / "checksums.sha256").read_text().splitlines()
            if line and not line.startswith("#")
        ]
        package = f"microsoft.ml.onnxruntime.{version}.nupkg"
        matching = [row for row in lines if len(row) >= 2 and row[1] == package]
        assert len(matching) == 1, (package, matching)
        assert re.fullmatch(r"[0-9a-f]{64}", matching[0][0])
        for rid in ["linux-x64", "linux-arm64"]:
            wheels = [
                row for row in lines if len(row) == 4 and row[2] == rid and row[1].startswith(f"onnxruntime-{version}-")
            ]
            assert len(wheels) == 1, (rid, wheels)
            assert re.fullmatch(r"[0-9a-f]{64}", wheels[0][0])

    def test_verifier_checks_bytes_and_rejects_zero_or_duplicate_pins(self):
        """Verify actual bytes and reject missing, duplicate or corrupted pins."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "sample.tgz"
            archive.write_bytes(b"known archive bytes")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            pins = root / "checksums"
            command = [_bash(), str(SCRIPTS / "verify-archive.sh"), str(archive), str(pins)]
            for entries, succeeds in [
                (f"{digest}  sample.tgz\n", True),
                ("", False),
                (f"{digest}  sample.tgz\n" * 2, False),
            ]:
                pins.write_text(entries)
                result = subprocess.run(command, capture_output=True, check=False)
                assert (result.returncode == 0) == succeeds, result.stderr.decode()
            pins.write_text(f"{digest}  sample.tgz\n")
            archive.write_bytes(b"changed archive bytes")
            assert subprocess.run(command, capture_output=True, check=False).returncode != 0

    def test_verified_mode_stages_the_package_and_ignores_the_cache(self):
        """Use only freshly verified package bytes, even with an older extracted tree."""
        self._check_verified_staging(tampered=False)

    def test_tampered_package_and_wheel_are_refused_before_staging(self):
        """Refuse a tampered package and a tampered wheel before creating any staged tree."""
        self._check_verified_staging(tampered=True)

    def test_unknown_version_is_refused_before_download(self):
        """Reject unsupported version requests before any provisioning."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = self._run_linux_setup(root, "0.0.0-unknown")
            assert result.returncode != 0, result.stdout + result.stderr
            assert "exactly one pinned SHA-256" in result.stderr
            assert not (root / "trace").exists()
            assert not (root / "workspace/libs").exists()
            assert not (root / "env").exists()
            assert not list(root.glob("onnxruntime-verified.*"))

    def _check_verified_staging(self, *, tampered: bool) -> None:
        version = "1.32.0"
        dirname = f"onnxruntime-linux-x64-{version}"
        package_name = f"microsoft.ml.onnxruntime.{version}.nupkg"
        wheel_name = f"onnxruntime-{version}-cp312-cp312-manylinux_2_28_x86_64.whl"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            old = root / "onnxruntime" / dirname / "lib"
            old.mkdir(parents=True)
            (old / "libonnxruntime.so").write_bytes(b"unverified cached library")

            package = root / "replacement"
            with zipfile.ZipFile(package, "w") as archive:
                archive.writestr("runtimes/linux-x64/native/libonnxruntime.so", b"verified replacement library")
                archive.writestr("build/native/include/onnxruntime_c_api.h", b"header")
            digest = hashlib.sha256(package.read_bytes()).hexdigest()
            (root / "checksums").write_text(
                f"{digest}  {package_name}\n"
                f"{'0' * 64}  {wheel_name}  linux-x64  https://files.pythonhosted.org/packages/aa/bb/{wheel_name}\n"
            )
            if tampered:
                package.write_bytes(b"tampered replacement")
            result = self._run_linux_setup(root, version)
            trace = (root / "trace").read_text().splitlines()
            error = result.stdout + result.stderr
            installed = root / "workspace/libs/libonnxruntime.so"
            if tampered:
                assert result.returncode == 1, error
                assert f"SHA-256 mismatch for {package_name}" in result.stderr, error
                assert f"SHA-256 mismatch for {wheel_name}" in result.stderr, error
                assert trace == ["curl", "curl"], trace
                assert not installed.exists()
                assert not (root / "env").exists()
                assert not list(root.glob("onnxruntime-verified.*"))
            else:
                assert result.returncode == 0, error
                assert f"{package_name}: SHA-256 verified" in result.stdout, error
                assert trace == ["curl"], trace
                assert installed.read_bytes() == b"verified replacement library"
                assert (old / "libonnxruntime.so").read_bytes() == b"unverified cached library"

    def _run_linux_setup(self, root: Path, version: str) -> subprocess.CompletedProcess[str]:
        tools = root / "bin"
        tools.mkdir()
        curl = tools / "curl"
        curl.write_text(
            """#!/bin/sh
set -eu
case "$*" in *--version*) exit 2 ;; esac
echo curl >> "$TEST_TRACE"
while [ "$1" != "-o" ]; do shift; done
cp "$TEST_REPLACEMENT" "$2"
"""
        )
        tar = tools / "tar"
        tar.write_text(
            """#!/bin/sh
set -eu
echo tar >> "$TEST_TRACE"
exec "$TEST_REAL_TAR" "$@"
"""
        )
        for tool in (curl, tar):
            tool.chmod(0o755)
        real_tar = shutil.which("tar")
        assert real_tar is not None
        env = {
            **os.environ,
            "PATH": os.pathsep.join((str(tools), os.environ["PATH"])),
            "RUNNER_TEMP": str(root),
            "GITHUB_WORKSPACE": str(root / "workspace"),
            "GITHUB_ENV": str(root / "env"),
            "TEST_TRACE": str(root / "trace"),
            "TEST_REPLACEMENT": str(root / "replacement"),
            "TEST_REAL_TAR": real_tar,
        }
        if (root / "checksums").exists():
            env["ORT_CHECKSUMS_FILE"] = str(root / "checksums")
        else:
            env.pop("ORT_CHECKSUMS_FILE", None)
        return subprocess.run(
            [_bash(), str(SCRIPTS / "linux.sh"), version, "libs", "x64", "system", "true"],
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )


if __name__ == "__main__":
    unittest.main()
