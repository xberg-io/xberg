"""Run the published raw-archive PowerShell entry points against synthetic PE files."""

import hashlib
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from collections.abc import Iterator
from pathlib import Path
from unittest import mock

import windows_wheel_closure as closure
from test_windows_wheel_closure import ROOT, expected_error, pe, windows_wheel_steps

PWSH = shutil.which("pwsh")
VERIFY = ROOT / "scripts/ci/verify-windows-dll-closure.ps1"
VENDOR = ROOT / "scripts/ci/vendor-windows-native-closure.ps1"


@unittest.skipUnless(PWSH, "PowerShell is required for the actual wrapper tests")
class RawWrapperTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.ort = self.root / "ort"
        self.ort.mkdir()
        self.crt = self.root / "Microsoft Visual Studio/2022/Enterprise/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
        self.crt.mkdir(parents=True)
        self.ort_binary = pe(delay=["msvcp140_1.dll"])
        self.msvcp = pe(["vcruntime140_1.dll"])
        self.vcruntime = pe(["KERNEL32.dll"])
        (self.ort / "onnxruntime.dll").write_bytes(self.ort_binary)
        (self.crt / "msvcp140_1.dll").write_bytes(self.msvcp)
        (self.crt / "vcruntime140_1.dll").write_bytes(self.vcruntime)

    def invoke(self, wrapper: Path, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [PWSH, "-NoProfile", "-NonInteractive", "-File", str(wrapper), *args],
            capture_output=True,
            text=True,
            env={**os.environ, "VCToolsRedistDir": str(self.crt)},
            check=False,
            timeout=30,
        )

    def archive(self, name: str, members: dict[str, bytes]) -> Path:
        path = self.root / name
        if path.suffix == ".zip":
            with zipfile.ZipFile(path, "w") as archive:
                for member, data in members.items():
                    archive.writestr(member, data)
        else:
            with tarfile.open(path, "w:gz") as archive:
                for member, data in members.items():
                    info = tarfile.TarInfo(member)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
        return path

    def unpack(self, artifact: Path) -> dict[str, bytes]:
        if artifact.suffix == ".zip":
            with zipfile.ZipFile(artifact) as archive:
                return {info.filename: archive.read(info) for info in archive.infolist() if not info.is_dir()}
        with tarfile.open(artifact) as archive:
            return {info.name: archive.extractfile(info).read() for info in archive.getmembers() if info.isfile()}

    def stage(self, artifact: Path, native: str) -> subprocess.CompletedProcess[str]:
        return self.invoke(VENDOR, "-Archive", str(artifact), "-DllSourceDir", str(self.ort), "-NativeGlob", native)

    def audit(self, artifact: Path, native: str, *extra: str) -> subprocess.CompletedProcess[str]:
        return self.invoke(VERIFY, "-Artifact", str(artifact), "-NativeGlob", native, "-ArtifactKind", "raw", *extra)

    def test_final_archive_gate_rejects_missing_transitive_delay_crt(self) -> None:
        artifact = self.archive(
            "cli.zip", {"bin/xberg.exe": pe(["onnxruntime.dll"]), "bin/onnxruntime.dll": self.ort_binary}
        )
        result = self.audit(artifact, "xberg.exe")
        assert result.returncode != 0, result.stdout
        assert "msvcp140_1.dll" in result.stderr

    def test_raw_archives_stage_and_audit_their_actual_packaged_files(self) -> None:
        layouts = {
            "php.zip": ("xberg.dll", ""),
            "cli.zip": ("xberg.exe", "xberg-cli-x86_64-pc-windows-msvc/"),
            "ffi.tar.gz": ("xberg_ffi.dll", "xberg-ffi-windows-x86_64/lib/"),
            "elixir.tar.gz": ("libxberg_nif-v1.3.10-nif-2.16-x86_64-pc-windows-msvc.dll", ""),
        }
        for filename, (native, prefix) in layouts.items():
            with self.subTest(filename=filename):
                original = {
                    prefix + native: pe(["onnxruntime.dll"]),
                    "LICENSE": b"original notice",
                    prefix + ".package-metadata": b"unchanged hidden metadata",
                }
                artifact = self.archive(filename, original)
                result = self.stage(artifact, native)
                assert result.returncode == 0, result.stderr
                files = self.unpack(artifact)
                assert {key: files[key] for key in original} == original
                assert files[prefix + "onnxruntime.dll"] == self.ort_binary
                assert files[prefix + "msvcp140_1.dll"] == self.msvcp
                assert files[prefix + "vcruntime140_1.dll"] == self.vcruntime
                notice = json.loads(files[prefix + "MICROSOFT_VC_RUNTIME.json"])
                assert {entry["sha256"] for entry in notice["files"]} == {
                    hashlib.sha256(self.msvcp).hexdigest(),
                    hashlib.sha256(self.vcruntime).hexdigest(),
                }
                result = self.audit(artifact, native, "-RequireCrtNotices")
                assert result.returncode == 0, result.stderr
                assert "Verified 4 PE files, 4 import edges, 1 native roots" in result.stdout

    def test_existing_crt_collision_does_not_replace_the_archive(self) -> None:
        artifact = self.archive(
            "collision.tgz", {"xberg.dll": pe(["onnxruntime.dll"]), "msvcp140_1.dll": pe(["KERNEL32.dll"])}
        )
        before = artifact.read_bytes()
        result = self.stage(artifact, "xberg.dll")
        assert result.returncode != 0, result.stdout
        assert "CRT byte collision" in result.stderr
        assert artifact.read_bytes() == before

    def test_unsafe_archive_member_is_rejected_before_replacement(self) -> None:
        artifact = self.archive("unsafe.zip", {"xberg.exe": pe(["onnxruntime.dll"]), "../escape.dll": pe()})
        before = artifact.read_bytes()
        result = self.stage(artifact, "xberg.exe")
        assert result.returncode != 0, result.stdout
        assert "unsafe artifact member" in result.stderr
        assert artifact.read_bytes() == before
        assert not (self.root / "escape.dll").exists()

    def test_zero_native_roots_is_a_hard_failure(self) -> None:
        artifact = self.archive("zero.zip", {"readme.txt": b"no binaries"})
        before = artifact.read_bytes()
        result = self.stage(artifact, "xberg.exe")
        assert result.returncode != 0, result.stdout
        assert artifact.read_bytes() == before
        assert self.audit(artifact, "xberg.exe").returncode != 0

    def test_stage_tree_then_verify_exact_final_archive(self) -> None:
        tree = self.root / "ffi"
        lib = tree / "lib"
        lib.mkdir(parents=True)
        native = "xberg_ffi.dll"
        binary = pe(["onnxruntime.dll"])
        (lib / native).write_bytes(binary)
        result = self.invoke(VENDOR, "-Artifact", str(tree), "-DllSourceDir", str(self.ort), "-NativeGlob", native)
        assert result.returncode == 0, result.stderr
        artifact = self.archive(
            "ffi.tar.gz", {f"ffi/lib/{member.name}": member.read_bytes() for member in lib.iterdir()}
        )
        result = self.audit(artifact, native, "-RequireCrtNotices")
        assert result.returncode == 0, result.stderr
        assert "Verified 4 PE files, 4 import edges, 1 native roots" in result.stdout
        assert self.unpack(artifact)["ffi/lib/xberg_ffi.dll"] == binary

    def test_final_archive_gate_rejects_lost_and_tampered_notices(self) -> None:
        artifact = self.archive("cli.zip", {"xberg.exe": pe(["onnxruntime.dll"])})
        result = self.stage(artifact, "xberg.exe")
        assert result.returncode == 0, result.stderr
        payload = self.unpack(artifact)
        notice_name = "MICROSOFT_VC_RUNTIME.json"
        notice = payload.pop(notice_name)
        missing = self.archive("no-notice.zip", payload)
        result = self.audit(missing, "xberg.exe", "-RequireCrtNotices")
        assert result.returncode != 0, result.stdout
        assert "missing Microsoft runtime notice" in result.stderr
        first_digest = json.loads(notice)["files"][0]["sha256"]
        payload[notice_name] = notice.replace(first_digest.encode(), b"0" * 64)
        tampered = self.archive("bad-notice.zip", payload)
        result = self.audit(tampered, "xberg.exe", "-RequireCrtNotices")
        assert result.returncode != 0, result.stdout
        assert "notice does not match shipped DLLs" in result.stderr

    def test_ort_source_cannot_supply_crt_when_licensed_source_is_missing(self) -> None:
        artifact = self.archive("elixir.tar.gz", {"libxberg_nif-test.dll": pe(["onnxruntime.dll"])})
        before = artifact.read_bytes()
        (self.crt / "msvcp140_1.dll").unlink()
        (self.ort / "msvcp140_1.dll").write_bytes(self.msvcp)
        result = self.stage(artifact, "libxberg_nif-*.dll")
        assert result.returncode != 0, result.stdout
        assert "msvcp140_1.dll" in result.stderr
        assert artifact.read_bytes() == before

    def test_directory_links_fail_before_audit_or_staging_can_follow_them(self) -> None:
        target = self.root / "external target"
        target.mkdir()
        payload = {"xberg.exe": pe(["onnxruntime.dll"]), "keep.txt": b"original"}
        for name, data in payload.items():
            (target / name).write_bytes(data)
        tree = self.root / "stage tree"
        tree.mkdir()
        (tree / "xberg.exe").write_bytes(payload["xberg.exe"])
        links = (self.root / "linked root", tree / "linked directory")
        for link in links:
            if os.name == "nt":
                result = subprocess.run(
                    ["cmd.exe", "/d", "/c", "mklink", "/J", str(link), str(target)],
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=30,
                )
                assert result.returncode == 0, result.stdout + result.stderr
                assert link.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT
                self.addCleanup(link.rmdir)
            else:
                link.symlink_to(target, target_is_directory=True)
                assert link.is_symlink()
        for artifact in (links[0], tree):
            with self.subTest(artifact=artifact):
                for result in (self.stage(artifact, "xberg.exe"), self.audit(artifact, "xberg.exe")):
                    assert result.returncode == 1, result
                    assert "artifact links are forbidden" in result.stderr
                assert {path.name: path.read_bytes() for path in target.iterdir()} == payload
                assert {path.name for path in tree.iterdir()} == {"xberg.exe", links[1].name}

    def test_reparse_flags_are_rejected_before_directory_iteration(self) -> None:
        tree = self.root / "ordinary tree"
        child = tree / "marked directory"
        child.mkdir(parents=True)
        (tree / "xberg.exe").write_bytes(pe(["onnxruntime.dll"]))
        real_iterdir = Path.iterdir
        for marked in (tree, child):
            with self.subTest(marked=marked):

                def attributes(path: Path) -> int:
                    return stat.FILE_ATTRIBUTE_REPARSE_POINT if path == marked else 0

                def checked_iterdir(path: Path) -> Iterator[Path]:
                    assert path != marked, "traversed a reparse point"
                    return real_iterdir(path)

                with (
                    mock.patch.object(closure, "_windows_file_attributes", side_effect=attributes, create=True),
                    mock.patch.object(Path, "iterdir", checked_iterdir),
                    expected_error(ValueError, "artifact links are forbidden"),
                ):
                    closure._raw_artifact_files(tree)

    def test_php_host_flags_reject_wrong_host_architecture(self) -> None:
        artifact = self.archive("php.zip", {"xberg.dll": pe(["php8.dll", "onnxruntime.dll"])})
        host = self.root / "php.exe"
        host.write_bytes(pe(["php8.dll"], machine=0xAA64))
        before = artifact.read_bytes()
        result = self.invoke(
            VENDOR,
            "-Archive",
            str(artifact),
            "-DllSourceDir",
            str(self.ort),
            "-NativeGlob",
            "xberg.dll",
            "-PhpHost",
            str(host),
            "-PhpVersion",
            "8.3",
        )
        assert result.returncode != 0, result.stdout
        assert "architecture" in result.stderr
        assert artifact.read_bytes() == before


def assert_raw_workflow(source: str, job: str, root: str, archive_argument: str) -> None:
    steps = windows_wheel_steps(source, job)
    tools = [
        index
        for index, step in enumerate(steps)
        if "sparse-checkout: |" in step and "scripts/ci/windows_raw_staging.py" in step
    ]
    vendor = [
        index
        for index, step in enumerate(steps)
        if "vendor-windows-native-closure.ps1" in step and "-DllSourceDir" in step
    ]
    audit = [
        index
        for index, step in enumerate(steps)
        if "verify-windows-dll-closure.ps1" in step and "-ArtifactKind raw" in step
    ]
    upload = [index for index, step in enumerate(steps) if "actions/upload-artifact@" in step]
    assert len(tools) == len(vendor) == len(audit) == 1 and upload, f"missing or duplicate release gate for {job}"
    assert tools[0] < vendor[0] <= audit[0] < upload[0], f"closure gate must precede upload for {job}"
    assert "ref: ${{ github.workflow_sha }}" in steps[tools[0]]
    assert "scripts/ci/windows_wheel_closure.py" in steps[tools[0]]
    assert "\n            mise.toml\n            mise.lock" in steps[tools[0]]
    python = steps[tools[0] + 1]
    assert tools[0] + 1 < vendor[0]
    assert "uses: xberg-io/actions/setup-mise@v1" in python
    assert "working-directory: .release-tools" in python and "tools: python" in python
    vendor_step = steps[vendor[0]]
    assert f'-NativeGlob "{root}"' in vendor_step
    vendor_call = next(line for line in vendor_step.splitlines() if "-DllSourceDir" in line)
    assert "ORT_DYLIB_PATH" in vendor_step and "ORT_LIB_LOCATION" not in vendor_call, (
        f"{job} must stage from the directory containing the selected ONNX Runtime DLL"
    )
    step = steps[audit[0]]
    conditions = [line.strip() for line in step.splitlines() if line.startswith("        if:")]
    assert archive_argument in step, f"final archive path is required for {job}"
    if job == "cli-binaries":
        assert not conditions
        assert step.index('if [[ "$RUNNER_OS" == "Windows" ]]') < step.index('7z a "${STAGE}.zip"')
        assert step.index('7z a "${STAGE}.zip"') < step.index(archive_argument)
    elif job == "c-ffi-libraries":
        assert conditions == ["if: contains(matrix.target, '-pc-windows-msvc')"]
        assert 'tar -czf "dist/c-ffi/${STAGE}.tar.gz"' in steps[vendor[0]]
    else:
        assert conditions == ["if: runner.os == 'Windows'"], f"unexpected release gate condition for {job}"
        assert "foreach ($archive in " in step
        if job == "php-extension":
            assert 'if ($archives.Count -eq 0) { throw "No Windows PHP archives to audit" }' in step
            assert step.count("-PhpHost $phpHost -PhpVersion $env:EXPECTED_PHP_VERSION") == 2
            assert step.index(archive_argument) < step.index("Get-FileHash")
        else:
            assert "if ($env:DRY_RUN -eq 'true')" in step
            assert 'throw "No Windows NIF archive to audit"' in step
    assert f'-NativeGlob "{root}" -ArtifactKind raw -RequireCrtNotices' in step


class RawWorkflowTests(unittest.TestCase):
    source = (ROOT / ".github/workflows/publish.yaml").read_text(encoding="utf-8")

    def test_publish_checks_each_exact_final_windows_archive(self) -> None:
        cases = [
            ("php-extension", "xberg.dll", "-Artifact $archive.FullName"),
            ("cli-binaries", "xberg.exe", '-Artifact "${STAGE}.zip"'),
            ("c-ffi-libraries", "xberg_ffi.dll", '-Artifact "dist/c-ffi/xberg-ffi-${{ matrix.platform }}.tar.gz"'),
            ("elixir-natives", "libxberg_nif-*.dll", "-Artifact $archive.FullName"),
        ]
        for job, root, artifact in cases:
            with self.subTest(job=job):
                assert_raw_workflow(self.source, job, root, artifact)

    def test_workflow_guard_rejects_audit_of_cli_staging_tree(self) -> None:
        archive = '-Artifact "${STAGE}.zip"'
        assert self.source.count(archive) == 1
        mutated = self.source.replace(archive, '-Artifact "$STAGE"')
        with expected_error(AssertionError, "final archive path is required"):
            assert_raw_workflow(mutated, "cli-binaries", "xberg.exe", archive)


MINIMUM_CLOSURE_TESTS = 96


def main() -> int:
    if PWSH is None:
        print("PowerShell is required for the Windows closure gate", file=sys.stderr)
        return 1
    suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern="test_windows*.py")
    collected = suite.countTestCases()
    if collected < MINIMUM_CLOSURE_TESTS:
        print(f"Windows closure gate collected {collected} of at least {MINIMUM_CLOSURE_TESTS} tests", file=sys.stderr)
        return 1
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return int(not result.wasSuccessful() or result.testsRun != collected or bool(result.skipped))


if __name__ == "__main__":
    sys.exit(main())
