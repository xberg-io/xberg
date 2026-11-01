"""Exercise the real Windows wheel gate with portable, valid PE fixtures."""

import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile
from collections.abc import Sequence
from pathlib import Path

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


class WheelClosureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "_xberg.pyd").write_bytes(pe(["onnxruntime.dll", "KERNEL32.dll", "python3.dll"]))
        (self.root / "onnxruntime.dll").write_bytes(pe(["MSVCP140_1.dll"]))

    def gate(self) -> subprocess.CompletedProcess[str]:
        command = [
            shutil.which("pwsh"),
            "-NoProfile",
            "-File",
            str(GATE),
            "-Artifact",
            str(getattr(self, "artifact", self.root)),
        ]
        return subprocess.run([*command, "-NativeGlob", "*.pyd"], capture_output=True, text=True, check=False)

    def require_failure(self, term: str) -> None:
        outcome = self.gate()
        assert outcome.returncode != 0, outcome.stdout + outcome.stderr
        assert term.lower() in (outcome.stdout + outcome.stderr).lower()

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
        self.require_failure("DirectML.dll")

    def test_malformed_transitive_pe_fails(self) -> None:
        (self.root / "onnxruntime.dll").write_bytes(b"not a PE")
        self.require_failure("PE")


class WheelVendoringTests(unittest.TestCase):
    def test_real_packager_stages_transitive_crt_and_record_notice(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            crt = root / "VS/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
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
                    shutil.which("pwsh"),
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
        self.crt = self.root / "VS/VC/Redist/MSVC/14.44.0/x64/Microsoft.VC143.CRT"
        self.crt.mkdir(parents=True)
        (self.crt / "MSVCP140_1.dll").write_bytes(pe(["VCRUNTIME140_1.dll"]))
        (self.crt / "VCRUNTIME140_1.dll").write_bytes(pe(["KERNEL32.dll"]))

    def stage(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts/ci/windows_wheel_closure.py"),
                "stage",
                "--artifact",
                str(self.artifact),
                "--crt-dir",
                str(self.crt),
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def failure(self, message: str) -> None:
        outcome = self.stage()
        assert outcome.returncode != 0, outcome.stdout + outcome.stderr
        assert message in outcome.stderr, outcome.stderr

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


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--write-pe":
        Path(sys.argv[2]).write_bytes(pe())
    else:
        unittest.main()
