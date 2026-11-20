"""Exercise the shipped-artifact x265 gate through its public command."""

from __future__ import annotations

import io
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify_no_gpl_codecs.py").resolve()

# A minimal ELF header followed by a payload. The gate only needs the magic and
# the marker bytes; it never parses the object.
ELF = b"\x7fELF\x02\x01\x01\x00"
MACHO = b"\xcf\xfa\xed\xfe\x07\x00\x00\x01"
PE = b"MZ\x90\x00"


class NoGplCodecsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def run_gate(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args], cwd=self.root, text=True, capture_output=True, check=False
        )

    def tar(self, name: str, members: dict[str, bytes]) -> Path:
        path = self.root / name
        with tarfile.open(path, "w:gz") as archive:
            for member, payload in members.items():
                info = tarfile.TarInfo(member)
                info.size = len(payload)
                archive.addfile(info, io.BytesIO(payload))
        return path

    def zip(self, name: str, members: dict[str, bytes]) -> Path:
        path = self.root / name
        with zipfile.ZipFile(path, "w") as archive:
            for member, payload in members.items():
                archive.writestr(member, payload)
        return path

    def test_should_reject_a_tarball_that_ships_a_fake_x265_library_name(self) -> None:
        path = self.tar("xberg-cli-test.tar.gz", {"stage/lib/libx265-fake.so.215": ELF})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "libx265-fake.so.215 is an x265-named file" in result.stderr

    def test_should_reject_a_binary_whose_needed_entry_names_libx265(self) -> None:
        path = self.tar("xberg-cli-test.tar.gz", {"stage/xberg": ELF + b"\x00libx265.so.215\x00"})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "stage/xberg references libx265" in result.stderr

    def test_should_reject_a_macho_binary_that_references_x265_symbols(self) -> None:
        path = self.tar("xberg-cli-darwin.tar.gz", {"stage/xberg": MACHO + b"\x00_x265_encoder_open\x00"})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "stage/xberg references x265_encoder_open" in result.stderr

    def test_should_reject_a_static_archive_that_references_x265_symbols(self) -> None:
        path = self.tar("xberg-ffi.tar.gz", {"stage/libxberg.a": b"!<arch>\n" + b"\x00" * 32 + b"x265_api\x00"})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "stage/libxberg.a references x265_api" in result.stderr

    def test_should_reject_a_windows_zip_that_references_x265_symbols(self) -> None:
        path = self.zip("xberg-cli-windows.zip", {"stage/xberg.exe": PE + b"\x00x265_api_get\x00"})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "stage/xberg.exe references x265_api_get" in result.stderr

    def test_should_reject_x265_in_a_staged_directory(self) -> None:
        stage = self.root / "stage"
        stage.mkdir()
        (stage / "lib").mkdir()
        (stage / "lib" / "libx265.215.dylib").write_bytes(MACHO)
        result = self.run_gate(str(stage))
        assert result.returncode == 1, result.stdout
        assert "libx265.215.dylib is an x265-named file" in result.stderr

    def test_should_accept_a_decode_only_archive_that_names_libx265_only_in_notices(self) -> None:
        notice = b"| libx265 | HEVC encode | GPL-2.0-or-later |\n"
        path = self.tar(
            "xberg-cli-test.tar.gz",
            {
                "stage/xberg": ELF + b"\x00libde265.so.0\x00de265_decode\x00",
                "stage/lib/libde265.so.0": ELF + b"\x00de265_decode\x00",
                "stage/lib/libaom.so.3": ELF + b"\x00aom_codec_av1_dx\x00",
                "stage/lib/libheif.so.1": ELF + b"\x00libde265.so.0\x00libaom.so.3\x00",
                "stage/THIRD_PARTY_LICENSES.md": notice,
            },
        )
        result = self.run_gate(str(path))
        assert result.returncode == 0, result.stderr
        assert "no x265 codec" in result.stdout

    def test_should_reject_an_artifact_that_yields_no_files(self) -> None:
        path = self.tar("xberg-cli-empty.tar.gz", {})
        result = self.run_gate(str(path))
        assert result.returncode == 1, result.stdout
        assert "yielded no files to scan" in result.stderr

    def test_should_reject_fewer_artifacts_than_the_asserted_count(self) -> None:
        path = self.tar("xberg-cli-test.tar.gz", {"stage/xberg": ELF})
        result = self.run_gate("--min-artifacts", "2", str(path))
        assert result.returncode == 1, result.stdout
        assert "examined 1 artifacts, expected at least 2" in result.stderr

    def test_should_reject_an_artifact_that_does_not_exist(self) -> None:
        result = self.run_gate(str(self.root / "missing.tar.gz"))
        assert result.returncode == 1, result.stdout
        assert "artifact not found" in result.stderr


if __name__ == "__main__":
    unittest.main()
