"""Exercise the release guard against complete and incomplete JNI artifacts."""

import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

GUARD = Path(__file__).with_name("verify-android-jni.py")
LIBRARY = "libxberg_jni.so"


def elf(machine: int) -> bytes:
    header = bytearray(64)
    header[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HH", header, 16, 3, machine)
    return bytes(header)


class AndroidJniGuardTests(unittest.TestCase):
    def check_guard(self, *args: Path | str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(GUARD), *map(str, args)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_staging_requires_both_matching_libraries_before_copying(self) -> None:
        for payload, expected in [
            (None, "No such file"),
            (b"", "ELF64"),
            (elf(62), "expected arm64-v8a"),
            (elf(183), ""),
        ]:
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / "artifacts"
                destination = Path(directory) / "jniLibs"
                for abi, data in [("x86_64", elf(62)), ("arm64-v8a", payload)]:
                    if data is not None:
                        path = source / abi / LIBRARY
                        path.parent.mkdir(parents=True)
                        path.write_bytes(data)
                result = self.check_guard("stage", source, destination)
                if expected:
                    assert result.returncode == 1, result.stderr
                    assert expected in result.stderr
                    assert not destination.exists()
                else:
                    assert result.returncode == 0, result.stderr
                    assert (destination / "arm64-v8a" / LIBRARY).read_bytes() == elf(183)
                    assert (destination / "x86_64" / LIBRARY).read_bytes() == elf(62)

    def test_aar_requires_both_matching_jni_entries(self) -> None:
        for payload, expected in [(None, "found 0"), (b"", "ELF64"), (elf(183), "expected x86_64"), (elf(62), "")]:
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as directory:
                aar = Path(directory) / "release.aar"
                with zipfile.ZipFile(aar, "w") as archive:
                    archive.writestr(f"jni/arm64-v8a/{LIBRARY}", elf(183))
                    if payload is not None:
                        archive.writestr(f"jni/x86_64/{LIBRARY}", payload)
                result = self.check_guard("verify-aar", aar)
                assert result.returncode == int(bool(expected)), result.stderr
                if expected:
                    assert expected in result.stderr


if __name__ == "__main__":
    unittest.main()
