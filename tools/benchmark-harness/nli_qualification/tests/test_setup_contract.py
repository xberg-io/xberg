import hashlib
from pathlib import Path
import subprocess
import tempfile
import shutil
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

    def test_each_nli_linux_archive_has_exactly_one_digest(self):
        """Require one actual checksum for each supported Linux architecture."""
        action = (ROOT / ".github/actions/setup-onnx-runtime/action.yml").read_text()
        workflow = (ROOT / ".github/workflows/ci-rust.yaml").read_text()
        version = version_sources(action, workflow)
        lines = [
            line.split()
            for line in (SCRIPTS / "checksums.sha256").read_text().splitlines()
            if line and not line.startswith("#")
        ]
        assert len(lines) == 2
        for arch in ["x64", "aarch64"]:
            matching = [row for row in lines if row[1] == f"onnxruntime-linux-{arch}-{version}.tgz"]
            assert len(matching) == 1
            assert re.fullmatch(r"[0-9a-f]{64}", matching[0][0])

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

    def test_corrupt_cached_archive_is_refused_before_extraction(self):
        """Reject corruption even when an older extracted library cache exists."""
        self._check_refused_archive("1.24.2", corrupt=True)

    def test_unknown_version_is_refused_before_download(self):
        """Reject unsupported version requests before any provisioning."""
        self._check_refused_archive("0.0.0-unknown", corrupt=False)

    def _check_refused_archive(self, version: str, *, corrupt: bool) -> None:
        import os

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "onnxruntime"
            cache.mkdir()
            if corrupt:
                (cache / f"onnxruntime-linux-x64-{version}.tgz").write_bytes(b"corrupt archive")
                old = cache / f"onnxruntime-linux-x64-{version}/lib"
                old.mkdir(parents=True)
                (old / "libonnxruntime.so").write_bytes(b"unverified cached library")
            env = {
                **os.environ,
                "RUNNER_TEMP": str(root),
                "GITHUB_WORKSPACE": str(root / "workspace"),
                "GITHUB_ENV": str(root / "env"),
            }
            result = subprocess.run(
                [_bash(), str(SCRIPTS / "linux.sh"), version, "libs", "x64", "system", "true"],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            assert result.returncode != 0, result.stdout + result.stderr
            assert not (root / "workspace/libs").exists()
            assert not list(root.glob("onnxruntime-verified.*"))
            if not corrupt:
                assert "exactly one pinned SHA-256" in result.stderr


if __name__ == "__main__":
    unittest.main()
