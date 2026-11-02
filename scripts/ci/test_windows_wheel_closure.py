"""Exercise the real Windows wheel gate with portable, valid PE fixtures."""

import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile
from collections.abc import Iterator, Sequence
from contextlib import contextmanager
from pathlib import Path
from unittest import mock

import windows_wheel_closure as closure

ROOT = Path(__file__).resolve().parents[2]
GATE = ROOT / "scripts/ci/verify-windows-dll-closure.ps1"


def pe(imports: Sequence[str] = (), delay: Sequence[str] = (), machine: int = 0x8664) -> bytearray:
    """Build a minimal PE import image without executing or shipping vendor code."""
    image = bytearray(4096)
    image[:2] = b"MZ"
    struct.pack_into("<I", image, 0x3C, 0x80)
    image[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<HHIIIHH", image, 0x84, machine, 1, 0, 0, 0, 240, 0x2022)
    struct.pack_into("<H", image, 0x98, 0x20B)
    struct.pack_into("<Q", image, 0x98 + 24, 0x180000000)
    struct.pack_into("<I", image, 0x98 + 108, 16)
    struct.pack_into("<8sIIII", image, 0x188, b".rdata", 3584, 0x1000, 3584, 512)
    cursor = 1536
    for index, names, size, start in [(1, imports, 20, 512), (13, delay, 32, 1024)]:
        if not names:
            continue
        struct.pack_into("<II", image, 0x98 + 112 + index * 8, 0x1000 + start - 512, size * (len(names) + 1))
        for offset, name in enumerate(names):
            encoded = name.encode() + b"\0"
            image[cursor : cursor + len(encoded)] = encoded
            rva = 0x1000 + cursor - 512
            if index == 1:
                struct.pack_into("<IIIII", image, start + offset * size, 1, 0, 0, rva, 1)
            else:
                struct.pack_into("<IIIIIIII", image, start + offset * size, 1, rva, 0, 0, 0, 0, 0, 0)
            cursor += len(encoded)
    return image


def require_pwsh() -> str:
    executable = shutil.which("pwsh")
    if not executable:
        raise unittest.SkipTest("pwsh is required for PowerShell wheel contract tests")
    return executable


@contextmanager
def expected_error(kind: type[Exception], message: str) -> Iterator[None]:
    try:
        yield
    except kind as error:
        detail = str(error)
    else:
        raise AssertionError(f"expected {kind.__name__}: {message}")
    assert message in detail, detail


class WheelClosureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "_xberg.pyd").write_bytes(pe(["onnxruntime.dll", "KERNEL32.dll", "python3.dll"]))
        (self.root / "onnxruntime.dll").write_bytes(pe(["MSVCP140_1.dll"]))

    def gate(self) -> subprocess.CompletedProcess[str]:
        command = [
            require_pwsh(),
            "-NoProfile",
            "-File",
            str(GATE),
            "-Artifact",
            str(getattr(self, "artifact", self.root)),
        ]
        return subprocess.run([*command, "-NativeGlob", "*.pyd"], capture_output=True, text=True, check=False)

    def require_failure(self, term: str) -> subprocess.CompletedProcess[str]:
        outcome = self.gate()
        assert outcome.returncode != 0, outcome.stdout + outcome.stderr
        assert term.lower() in (outcome.stdout + outcome.stderr).lower()
        return outcome

    def test_missing_transitive_runtime_fails_even_when_ort_is_present(self) -> None:
        self.require_failure("MSVCP140_1.dll")

    def test_complete_valid_pe_graph_succeeds_and_counts_real_work(self) -> None:
        (self.root / "MSVCP140_1.dll").write_bytes(pe(["VCRUNTIME140_1.dll"]))
        (self.root / "VCRUNTIME140_1.dll").write_bytes(pe(["KERNEL32.dll"]))
        outcome = self.gate()
        assert outcome.returncode == 0, outcome.stdout + outcome.stderr
        assert "4 PE files" in outcome.stdout

    def test_second_missing_edge_is_not_hidden_by_first_supplied_runtime(self) -> None:
        (self.root / "MSVCP140_1.dll").write_bytes(pe(["VCRUNTIME140_1.dll"]))
        self.require_failure("VCRUNTIME140_1.dll")

    def test_delay_imports_are_required(self) -> None:
        (self.root / "onnxruntime.dll").write_bytes(pe(delay=["MSVCP140_1.dll"]))
        self.require_failure("MSVCP140_1.dll")

    def test_wrong_machine_fails(self) -> None:
        (self.root / "MSVCP140_1.dll").write_bytes(pe(machine=0xAA64))
        self.require_failure("architecture")

    def test_case_insensitive_basename_collision_fails(self) -> None:
        self.artifact = self.root / "wheel.zip"
        with zipfile.ZipFile(self.artifact, "w") as archive:
            for path in self.root.glob("*.pyd"):
                archive.writestr(path.name, path.read_bytes())
            archive.writestr("onnxruntime.dll", pe())
            archive.writestr("msvcp140_1.dll", pe())
            archive.writestr("MSVCP140_1.dll", pe(["OTHER.dll"]))
        self.require_failure("collision")

    def test_empty_input_fails(self) -> None:
        (self.root / "_xberg.pyd").unlink()
        self.require_failure("no native")

    def test_forbidden_import_in_transitive_dll_fails(self) -> None:
        (self.root / "onnxruntime.dll").write_bytes(pe(["DirectML.dll"]))
        outcome = self.require_failure("DirectML.dll")
        assert "imports forbidden directml.dll" in outcome.stderr.lower(), outcome.stderr

    def test_malformed_transitive_pe_fails(self) -> None:
        (self.root / "onnxruntime.dll").write_bytes(b"not a PE")
        self.require_failure("PE")

    def test_shipped_directml_is_still_rejected_for_the_forbidden_import(self) -> None:
        (self.root / "onnxruntime.dll").write_bytes(pe(["DirectML.dll"]))
        (self.root / "DirectML.dll").write_bytes(pe())
        outcome = self.require_failure("DirectML.dll")
        assert "imports forbidden directml.dll" in outcome.stderr.lower(), outcome.stderr

    def test_pe_outside_the_native_search_directory_is_rejected(self) -> None:
        (self.root / "MSVCP140_1.dll").write_bytes(pe())
        outside = self.root / "other"
        outside.mkdir()
        (outside / "auxiliary.dll").write_bytes(pe())
        self.require_failure("PE file outside native search directory: other/auxiliary.dll")

    def test_missing_pwsh_is_reported_as_an_explicit_skip(self) -> None:
        with mock.patch("shutil.which", return_value=None), expected_error(unittest.SkipTest, "pwsh is required"):
            self.gate()


class WheelVendoringTests(unittest.TestCase):
    def test_real_packager_stages_transitive_crt_and_record_notice(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            crt = root / "Microsoft Visual Studio/2022/Enterprise/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
            crt.mkdir(parents=True)
            (crt / "MSVCP140_1.dll").write_bytes(pe(["VCRUNTIME140_1.dll"]))
            (crt / "VCRUNTIME140_1.dll").write_bytes(pe(["KERNEL32.dll"]))
            ort = root / "ort"
            ort.mkdir()
            (ort / "onnxruntime.dll").write_bytes(pe(["MSVCP140_1.dll"]))
            wheel = root / "test.whl"
            with zipfile.ZipFile(wheel, "w") as archive:
                archive.writestr("xberg/_xberg.pyd", pe(["onnxruntime.dll"]))
                archive.writestr("test.dist-info/RECORD", "xberg/_xberg.pyd,,\n")
            outcome = subprocess.run(
                [
                    require_pwsh(),
                    "-NoProfile",
                    "-File",
                    str(ROOT / "scripts/ci/vendor-windows-wheel-closure.ps1"),
                    str(wheel),
                    str(ort),
                    "*.pyd",
                ],
                env={**os.environ, "VCToolsRedistDir": str(crt.parent.parent)},
                capture_output=True,
                text=True,
                check=False,
            )
            assert outcome.returncode == 0, outcome.stdout + outcome.stderr
            with zipfile.ZipFile(wheel) as archive:
                assert "xberg/MSVCP140_1.dll" in archive.namelist()
                assert archive.read("xberg/VCRUNTIME140_1.dll") == (crt / "VCRUNTIME140_1.dll").read_bytes()
                record = archive.read("test.dist-info/RECORD").decode()
                assert "xberg/MSVCP140_1.dll,sha256=" in record
                assert "licenses/MICROSOFT_VC_RUNTIME.json,sha256=" in record
                notice = json.loads(archive.read("test.dist-info/licenses/MICROSOFT_VC_RUNTIME.json"))
                for item in notice["files"]:
                    assert hashlib.sha256(archive.read(item["file"])).hexdigest() == item["sha256"]


class RuntimeSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.artifact = self.root / "artifact"
        self.artifact.mkdir()
        (self.artifact / "test.dist-info").mkdir()
        (self.artifact / "_xberg.pyd").write_bytes(pe(["onnxruntime.dll"]))
        (self.artifact / "onnxruntime.dll").write_bytes(pe(["MSVCP140_1.dll"]))
        self.crt = self.root / "Microsoft Visual Studio/2022/Enterprise/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
        self.crt.mkdir(parents=True)
        (self.crt / "MSVCP140_1.dll").write_bytes(pe(["VCRUNTIME140_1.dll"]))
        (self.crt / "VCRUNTIME140_1.dll").write_bytes(pe(["KERNEL32.dll"]))

    def stage(self, *, environment_override: bool = False) -> subprocess.CompletedProcess[str]:
        command = [
            sys.executable,
            str(ROOT / "scripts/ci/windows_wheel_closure.py"),
            "stage",
            "--artifact",
            str(self.artifact),
        ]
        environment = dict(os.environ)
        if environment_override:
            environment["VCToolsRedistDir"] = str(self.crt)
        else:
            command.extend(["--crt-dir", str(self.crt)])
        return subprocess.run(command, env=environment, capture_output=True, text=True, check=False)

    def failure(self, message: str) -> None:
        outcome = self.stage()
        assert outcome.returncode != 0, outcome.stdout + outcome.stderr
        assert message in outcome.stderr, outcome.stderr

    def move_source(self, year: str, edition: str) -> None:
        target = self.root / f"Microsoft Visual Studio/{year}/{edition}/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
        target.parent.mkdir(parents=True, exist_ok=True)
        self.crt.rename(target)
        self.crt = target
        assert sorted(path.name for path in self.crt.iterdir()) == ["MSVCP140_1.dll", "VCRUNTIME140_1.dll"]

    def reject_source(self, year: str, edition: str, *, environment_override: bool) -> None:
        self.move_source(year, edition)
        before = {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}
        outcome = self.stage(environment_override=environment_override)
        assert outcome.returncode != 0, outcome.stdout + outcome.stderr
        assert "release CRT redist" in outcome.stderr, outcome.stderr
        assert before == {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}

    @contextmanager
    def vswhere_result(self, output: str = "", error: OSError | None = None) -> Iterator[mock.MagicMock]:
        result = subprocess.CompletedProcess([], 0, stdout=output, stderr="")
        with (
            mock.patch.dict(os.environ, {"VCToolsRedistDir": ""}),
            mock.patch.object(closure.subprocess, "run", return_value=result, side_effect=error) as lookup,
            mock.patch.object(Path, "resolve", autospec=True, side_effect=Path.resolve) as resolution,
        ):
            try:
                yield resolution
            finally:
                lookup.assert_called_once()
                command = lookup.call_args.args[0]
                assert command[command.index("-version") + 1] == "[17.0,18.0)"
                for edition in closure.VS2022_EDITIONS:
                    assert f"Microsoft.VisualStudio.Product.{edition}" in command

    def test_empty_vswhere_result_names_vs2022_before_resolving_paths(self) -> None:
        with self.vswhere_result() as resolution:
            with expected_error(ValueError, "Visual Studio 2022"):
                closure.crt_directory(0x8664, None)
            resolution.assert_not_called()

    def test_non_vs2022_lookup_is_rejected_before_resolving_paths(self) -> None:
        self.move_source("2026", "Enterprise")
        installation = self.root / "Microsoft Visual Studio/2026/Enterprise"
        with self.vswhere_result(str(installation)) as resolution:
            with expected_error(ValueError, "Visual Studio 2022"):
                closure.crt_directory(0x8664, None)
            resolution.assert_not_called()

    def test_unavailable_vswhere_names_vs2022(self) -> None:
        with self.vswhere_result(error=FileNotFoundError("vswhere unavailable")) as resolution:
            with expected_error(ValueError, "Visual Studio 2022"):
                closure.crt_directory(0x8664, None)
            resolution.assert_not_called()

    def test_supported_vs2022_lookup_resolves_the_actual_release_crt(self) -> None:
        installation = self.root / "Microsoft Visual Studio/2022/Enterprise"
        expected = self.crt.resolve()
        with self.vswhere_result(str(installation) + "\n") as resolution:
            assert closure.crt_directory(0x8664, None) == expected
            resolution.assert_called_once()

    def test_stage_final_audit_rejects_shipped_forbidden_dll_before_any_writes(self) -> None:
        (self.artifact / "onnxruntime.dll").write_bytes(pe(["MSVCP140_1.dll", "DirectML.dll"]))
        (self.artifact / "DirectML.dll").write_bytes(pe())
        before = {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}
        self.failure("imports forbidden directml.dll")
        assert before == {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}

    def test_explicit_buildtools_source_is_rejected_before_writes(self) -> None:
        self.reject_source("2022", "BuildTools", environment_override=False)

    def test_environment_buildtools_source_is_rejected_before_writes(self) -> None:
        self.reject_source("2022", "BuildTools", environment_override=True)

    def test_explicit_unsupported_visual_studio_version_is_rejected_before_writes(self) -> None:
        self.reject_source("2026", "Enterprise", environment_override=False)

    def test_environment_unsupported_visual_studio_version_is_rejected_before_writes(self) -> None:
        self.reject_source("2026", "Enterprise", environment_override=True)

    def test_other_supported_full_editions_keep_the_same_release_source_contract(self) -> None:
        for edition in ["Community", "Professional"]:
            self.move_source("2022", edition)
            outcome = self.stage(environment_override=edition == "Professional")
            assert outcome.returncode == 0, outcome.stdout + outcome.stderr
            assert (self.artifact / "MSVCP140_1.dll").read_bytes() == (self.crt / "MSVCP140_1.dll").read_bytes()

    def test_missing_transitive_source_does_not_write_partial_runtime(self) -> None:
        (self.crt / "VCRUNTIME140_1.dll").unlink()
        self.failure("unavailable")
        assert sorted(path.name for path in self.artifact.glob("*.dll")) == ["onnxruntime.dll"]

    def test_system32_is_not_a_redistribution_source(self) -> None:
        self.crt = self.root / "System32"
        self.crt.mkdir()
        self.failure("release CRT redist")

    def test_release_crt_machine_must_match_real_importer(self) -> None:
        (self.crt / "MSVCP140_1.dll").write_bytes(pe(machine=0xAA64))
        self.failure("architecture")

    def test_existing_runtime_with_different_bytes_is_not_silently_kept(self) -> None:
        (self.artifact / "MSVCP140_1.dll").write_bytes(pe())
        self.failure("collision")

    def test_staging_is_idempotent_with_exact_notice_bytes(self) -> None:
        first = self.stage()
        assert first.returncode == 0, first.stderr
        before = {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}
        second = self.stage()
        assert second.returncode == 0, second.stderr
        assert json.loads(second.stdout)["added"] == []
        assert before == {str(path): path.read_bytes() for path in self.artifact.rglob("*") if path.is_file()}


def windows_wheel_steps(source: str, job: str) -> list[str]:
    match = re.search(rf"(?ms)^  {re.escape(job)}:\n(.*?)(?=^  [\w-]+:|\Z)", source)
    assert match is not None, f"missing workflow job: {job}"
    return re.split(r"(?m)^      - ", match[1])[1:]


def assert_windows_wheel_wiring(source: str, job: str) -> None:
    steps = windows_wheel_steps(source, job)
    selected = []
    for label, script, argument in [
        ("vendor", "vendor-windows-wheel-closure.ps1", "-Wheel"),
        ("verify", "verify-windows-dll-closure.ps1", "-Artifact"),
    ]:
        matching = [(index, step) for index, step in enumerate(steps) if script in step]
        assert len(matching) == 1, f"expected exactly one Windows wheel {label} step; found {len(matching)}"
        index, step = matching[0]
        assert "if: runner.os == 'Windows'" in step, f"{label} must run on Windows"
        assert "shell: pwsh" in step, f"{label} must use PowerShell"
        assert 'Get-ChildItem -Path wheelhouse -Filter "*.whl" | ForEach-Object {' in step
        lines = [line.strip() for line in step.splitlines()]
        calls = [index for index, line in enumerate(lines) if line.startswith("& ") and f'\\{script}"' in line]
        assert len(calls) == 1, f"expected one active {label} invocation; found {len(calls)}"
        call = calls[0]
        assert f"{argument} $_.FullName" in lines[call], f"missing {label} wheel argument"
        assert '-NativeGlob "*.pyd"' in lines[call], f"{label} must target wheel extensions"
        guard = f'if ($LASTEXITCODE -ne 0) {{ throw "{script.removesuffix(".ps1")} failed'
        assert call + 1 < len(lines) and lines[call + 1].startswith(guard), f"missing {label} exit guard"
        selected.append(index)
    assert selected[0] < selected[1], "verify must follow vendoring"


class WheelWorkflowTests(unittest.TestCase):
    def test_publish_runs_both_windows_wheel_closure_steps(self) -> None:
        source = (ROOT / ".github/workflows/publish.yaml").read_text()
        assert_windows_wheel_wiring(source, "python-wheels")

    def test_smoke_ci_runs_both_windows_wheel_closure_steps(self) -> None:
        source = (ROOT / ".github/workflows/ci-python-wheel-smoke.yaml").read_text()
        assert_windows_wheel_wiring(source, "python-wheel-smoke")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--write-pe":
        Path(sys.argv[2]).write_bytes(pe())
    else:
        unittest.main()
