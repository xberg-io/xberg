#!/usr/bin/env python3
"""Contract tests for ``scripts/ci/verify_windows_heif_archive.py``.

Each case builds a throwaway Windows CLI archive so the gate is exercised on the
bytes the release job produces: a dynamically linked archive with its licence
texts passes, and a statically linked one, a missing or placeholder licence
text, an undocumented link or an incomplete DLL closure each fail.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

from test_windows_wheel_closure import pe

SCRIPT = Path(__file__).with_name("verify_windows_heif_archive.py")
ARCHIVE_NAME = "xberg-cli-x86_64-pc-windows-msvc.zip"
STAGED = f"{ARCHIVE_NAME[:-4]}/"

LGPL_TEXT = b"                    GNU LESSER GENERAL PUBLIC LICENSE\n                       Version 3, 29 June 2007\n"
GPL_TEXT = b"                    GNU GENERAL PUBLIC LICENSE\n                       Version 3, 29 June 2007\n"
AOM_NOTICE = b"Copyright (c) 2016, Alliance for Open Media. All rights reserved.\n"
PROJECT_NOTICE = b"# Third-Party Licenses\n\nThe Windows CLI links heif.dll and libde265.dll dynamically.\n"

DYNAMIC_FILES = {
    "xberg.exe": bytes(pe(["heif.dll", "onnxruntime.dll", "KERNEL32.dll"])),
    "heif.dll": bytes(pe(["libde265.dll", "aom.dll", "MSVCP140.dll", "KERNEL32.dll"])),
    "libde265.dll": bytes(pe(["KERNEL32.dll"])),
    "aom.dll": bytes(pe(["KERNEL32.dll"])),
    "onnxruntime.dll": bytes(pe(["KERNEL32.dll"])),
    "MSVCP140.dll": bytes(pe(["KERNEL32.dll"])),
    "LICENSE": b"MIT License\n",
    "THIRD_PARTY_LICENSES.md": PROJECT_NOTICE,
    "LGPL-3.0.txt": LGPL_TEXT,
    "GPL-3.0.txt": GPL_TEXT,
    "libheif-COPYING": LGPL_TEXT,
    "libde265-COPYING": LGPL_TEXT,
    "aom-COPYING": AOM_NOTICE,
}


def run_gate(directory: Path, files: dict[str, bytes]) -> subprocess.CompletedProcess[str]:
    """Build an archive in ``directory`` and run the gate against it."""
    archive = directory / ARCHIVE_NAME
    with zipfile.ZipFile(archive, "w") as handle:
        for name, payload in files.items():
            handle.writestr(f"{STAGED}{name}", payload)
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--archive", str(archive)],
        capture_output=True,
        text=True,
        check=False,
    )


def without(*names: str) -> dict[str, bytes]:
    """Return a valid archive payload minus the named members."""
    return {name: payload for name, payload in DYNAMIC_FILES.items() if name not in names}


def case_dynamic_archive_passes() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), DYNAMIC_FILES)
    assert result.returncode == 0, result.stderr
    assert "links libheif dynamically" in result.stdout, result.stdout


def case_static_link_is_reported() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(
            Path(tmp),
            {
                "xberg.exe": bytes(pe(["onnxruntime.dll", "KERNEL32.dll"])) + b" libheif heif_image_create",
                "onnxruntime.dll": bytes(pe(["KERNEL32.dll"])),
                **{
                    name: payload
                    for name, payload in DYNAMIC_FILES.items()
                    if name not in {"xberg.exe", "onnxruntime.dll"}
                },
            },
        )
    assert result.returncode == 1, result.stdout
    assert "does not import heif.dll or libheif.dll" in result.stderr, result.stderr
    assert "section 4(d)(0)" in result.stderr, result.stderr


def case_libheif_prefixed_name_is_accepted() -> None:
    libheif = bytes(pe(["libde265.dll", "aom.dll", "MSVCP140.dll", "KERNEL32.dll"]))
    files = {name: payload for name, payload in DYNAMIC_FILES.items() if name != "heif.dll"}
    files["libheif.dll"] = libheif
    files["xberg.exe"] = bytes(pe(["libheif.dll", "onnxruntime.dll", "KERNEL32.dll"]))
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), files)
    assert result.returncode == 0, result.stderr


def case_imported_dll_missing_under_its_own_name_fails() -> None:
    files = {**DYNAMIC_FILES, "libheif.dll": DYNAMIC_FILES["heif.dll"]}
    del files["heif.dll"]
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), files)
    assert result.returncode == 1, result.stdout
    assert "xberg.exe imports heif.dll but the archive does not carry it" in result.stderr, result.stderr


def case_missing_codec_dll_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), without("libde265.dll"))
    assert result.returncode == 1, result.stdout
    assert "does not carry libde265.dll" in result.stderr, result.stderr
    assert "missing wheel-local dependency libde265.dll" in result.stderr, result.stderr


def case_missing_lgpl_text_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), without("LGPL-3.0.txt"))
    assert result.returncode == 1, result.stdout
    assert "missing the LGPL-3.0.txt license text" in result.stderr, result.stderr


def case_placeholder_license_text_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {**DYNAMIC_FILES, "LGPL-3.0.txt": b"see the website\n"})
    assert result.returncode == 1, result.stdout
    assert "LGPL-3.0.txt does not contain GNU LESSER GENERAL PUBLIC LICENSE" in result.stderr, result.stderr


def case_missing_codec_notice_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), without("aom-COPYING"))
    assert result.returncode == 1, result.stdout
    assert "missing the aom-COPYING license text" in result.stderr, result.stderr


def case_undocumented_dynamic_link_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {**DYNAMIC_FILES, "THIRD_PARTY_LICENSES.md": b"# Third-Party Licenses\n"})
    assert result.returncode == 1, result.stdout
    assert "THIRD_PARTY_LICENSES.md does not document heif.dll" in result.stderr, result.stderr


def case_non_system_import_outside_the_archive_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        files = {
            **DYNAMIC_FILES,
            "heif.dll": bytes(pe(["libde265.dll", "aom.dll", "MSVCP140.dll", "libwinpthread-1.dll"])),
        }
        result = run_gate(Path(tmp), files)
    assert result.returncode == 1, result.stdout
    assert "missing wheel-local dependency libwinpthread-1.dll" in result.stderr, result.stderr


def case_missing_archive_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--archive", str(Path(tmp) / "absent.zip")],
            capture_output=True,
            text=True,
            check=False,
        )
    assert result.returncode == 1, result.stdout
    assert "archive not found" in result.stderr, result.stderr


def case_archive_without_executable_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {"LICENSE": b"MIT"})
    assert result.returncode == 1, result.stdout
    assert "holds no executable" in result.stderr, result.stderr


CASES = (
    case_dynamic_archive_passes,
    case_static_link_is_reported,
    case_libheif_prefixed_name_is_accepted,
    case_imported_dll_missing_under_its_own_name_fails,
    case_missing_codec_dll_fails,
    case_missing_lgpl_text_fails,
    case_placeholder_license_text_fails,
    case_missing_codec_notice_fails,
    case_undocumented_dynamic_link_fails,
    case_non_system_import_outside_the_archive_fails,
    case_missing_archive_fails,
    case_archive_without_executable_fails,
)


def main() -> int:
    for case in CASES:
        case()
        print(f"ok  {case.__name__}")
    print(f"{len(CASES)} Windows HEIF archive gate cases passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
