"""Exercise Android AAR notice injection against small synthetic archives."""

from __future__ import annotations

import struct
import subprocess
import sys
import tempfile
import unittest
import warnings
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("android-aar-notices.py").resolve()
ROOT = SCRIPT.parents[2]
NOTICES = ("LICENSE", "THIRD_PARTY_LICENSES.md")
MANIFEST = "AndroidManifest.xml"


def notice_bytes(name: str) -> bytes:
    return (ROOT / name).read_bytes()


def elf(machine: int) -> bytes:
    header = bytearray(64)
    header[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HH", header, 16, 3, machine)
    struct.pack_into("<I", header, 20, 1)
    struct.pack_into("<Q", header, 32, 64)
    struct.pack_into("<3H", header, 52, 64, 56, 1)
    segment = struct.pack("<II6Q", 1, 5, 120, 120, 0, 8, 8, 8)
    return bytes(header) + segment + bytes(8)


PAYLOAD = {
    MANIFEST: b'<manifest package="io.xberg"/>',
    "classes.jar": b"PK\x03\x04classes",
    "jni/arm64-v8a/libxberg_jni.so": elf(183),
    "jni/x86_64/libxberg_jni.so": elf(62),
    "R.txt": b"",
}


def build_aar(
    directory: Path, *, notices: dict[str, bytes] | None = None, extra: dict[str, bytes] | None = None
) -> Path:
    path = directory / "xberg-android-release.aar"
    payload = dict(PAYLOAD)
    payload.update(notices or {})
    payload.update(extra or {})
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in payload.items():
            archive.writestr(name, content)
    return path


def run_script(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )


def entries(path: Path) -> list[tuple[str, bytes]]:
    with zipfile.ZipFile(path) as archive:
        names = [entry.filename for entry in archive.infolist()]
        assert len(names) == len(set(names)), names
        return [(name, archive.read(name)) for name in names]


class AndroidAarNoticesTests(unittest.TestCase):
    def test_injection_adds_exact_canonical_bytes_and_keeps_every_other_entry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(Path(directory))
            before = entries(aar)
            result = run_script("inject", "--canonical-root", str(ROOT), "--expected-count", "1", str(aar))
            assert result.returncode == 0, result.stderr
            assert f"Injected 2 canonical notices into {aar}" in result.stdout
            after = entries(aar)
            assert after[: len(before)] == before
            injected = dict(after[len(before) :])
            assert list(injected) == ["META-INF/LICENSE", "META-INF/THIRD_PARTY_LICENSES.md"]
            for name in NOTICES:
                assert injected[f"META-INF/{name}"] == notice_bytes(name)
            assert dict(after)[MANIFEST] == PAYLOAD[MANIFEST]

    def test_injection_is_idempotent_and_replaces_a_stale_notice(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(
                Path(directory),
                notices={"META-INF/LICENSE": b"stale", "META-INF/THIRD_PARTY_LICENSES.md": b"stale"},
            )
            for _ in range(2):
                result = run_script("inject", "--canonical-root", str(ROOT), str(aar))
                assert result.returncode == 0, result.stderr
            packaged = dict(entries(aar))
            assert [name for name in packaged if name.startswith("META-INF/")] == [
                "META-INF/LICENSE",
                "META-INF/THIRD_PARTY_LICENSES.md",
            ]
            for name in NOTICES:
                assert packaged[f"META-INF/{name}"] == notice_bytes(name)

    def test_verify_accepts_the_injected_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(Path(directory))
            assert run_script("inject", "--canonical-root", str(ROOT), str(aar)).returncode == 0
            result = run_script("verify", "--canonical-root", str(ROOT), "--expected-count", "1", str(aar))
            assert result.returncode == 0, result.stderr
            assert f"Verified 2 canonical notices in {aar}" in result.stdout

    def test_verify_names_the_archive_and_missing_notice(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(Path(directory), notices={"META-INF/LICENSE": notice_bytes("LICENSE")})
            result = run_script("verify", "--canonical-root", str(ROOT), str(aar))
            assert result.returncode == 1
            assert f"{aar}: expected one META-INF/THIRD_PARTY_LICENSES.md, found 0" in result.stderr
            assert "Verified" not in result.stdout

    def test_verify_names_the_archive_and_truncated_notice(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(
                Path(directory),
                notices={
                    "META-INF/LICENSE": notice_bytes("LICENSE"),
                    "META-INF/THIRD_PARTY_LICENSES.md": b"truncated",
                },
            )
            result = run_script("verify", "--canonical-root", str(ROOT), str(aar))
            assert result.returncode == 1
            assert (
                f"{aar}: META-INF/THIRD_PARTY_LICENSES.md differs from canonical THIRD_PARTY_LICENSES.md "
                f"({len(b'truncated')} bytes, expected {len(notice_bytes('THIRD_PARTY_LICENSES.md'))})"
            ) in result.stderr
            assert "Verified" not in result.stdout

    def test_inject_rejects_unreadable_and_malformed_aars(self) -> None:
        cases = {
            "empty": (b"", "empty AAR"),
            "not-a-zip": (b"nothing to see", "not a ZIP archive"),
        }
        with tempfile.TemporaryDirectory() as directory:
            for name, (payload, expected) in cases.items():
                path = Path(directory) / f"{name}.aar"
                path.write_bytes(payload)
                result = run_script("inject", "--canonical-root", str(ROOT), str(path))
                assert result.returncode == 1, name
                assert expected in result.stderr, (name, result.stderr)
            missing = Path(directory) / "absent.aar"
            result = run_script("inject", "--canonical-root", str(ROOT), str(missing))
            assert result.returncode == 1
            assert f"AAR not found: {missing}" in result.stderr

    def test_injection_requires_the_manifest_at_the_archive_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            directory_path = Path(directory)
            nested = directory_path / "nested.aar"
            with zipfile.ZipFile(nested, "w") as archive:
                archive.writestr(f"res/{MANIFEST}", PAYLOAD[MANIFEST])
            duplicated = directory_path / "duplicated.aar"
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", UserWarning)
                with zipfile.ZipFile(duplicated, "w") as archive:
                    archive.writestr(MANIFEST, PAYLOAD[MANIFEST])
                    archive.writestr(MANIFEST, PAYLOAD[MANIFEST])
            for path in (nested, duplicated):
                result = run_script("inject", "--canonical-root", str(ROOT), str(path))
                assert result.returncode == 1, path.name
                assert f"{path}: expected one {MANIFEST} at the archive root" in result.stderr

    def test_injection_rejects_a_signed_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(Path(directory), extra={"META-INF/CERT.SF": b"signature"})
            result = run_script("inject", "--canonical-root", str(ROOT), str(aar))
            assert result.returncode == 1
            assert f"{aar}: archive is signed" in result.stderr
            assert "invalidate META-INF/CERT.SF" in result.stderr

    def test_a_stale_or_empty_canonical_bundle_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            directory_path = Path(directory)
            aar = build_aar(directory_path)
            canonical = directory_path / "canonical"
            canonical.mkdir()
            (canonical / "LICENSE").write_bytes(notice_bytes("LICENSE"))
            (canonical / "THIRD_PARTY_LICENSES.md").write_bytes(b"# hand-written\n")
            result = run_script("inject", "--canonical-root", str(canonical), str(aar))
            assert result.returncode == 1
            assert "THIRD_PARTY_LICENSES.md is not the generated notice bundle" in result.stderr
            (canonical / "THIRD_PARTY_LICENSES.md").write_bytes(notice_bytes("THIRD_PARTY_LICENSES.md"))
            (canonical / "LICENSE").write_bytes(b"")
            result = run_script("inject", "--canonical-root", str(canonical), str(aar))
            assert result.returncode == 1
            assert "empty canonical notice: LICENSE" in result.stderr

    def test_the_archive_count_must_match_and_never_be_zero(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            aar = build_aar(Path(directory))
            result = run_script("verify", "--canonical-root", str(ROOT), "--expected-count", "2", str(aar))
            assert result.returncode == 1
            assert "expected 2 AAR archives, found 1" in result.stderr
            result = run_script("verify", "--canonical-root", str(ROOT), "--expected-count", "0", str(aar))
            assert result.returncode == 1
            assert "expected at least one AAR archive" in result.stderr


if __name__ == "__main__":
    unittest.main()
