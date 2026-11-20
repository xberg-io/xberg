#!/usr/bin/env python3
"""Contract tests for ``scripts/ci/verify_no_gpl_cli_archive.py``.

Each case builds a throwaway archive so the gate is exercised on the bytes it
will see in the release job: a clean executable passes, an executable carrying a
codec symbol fails, and a binary that lost PDF or Office extraction fails.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify_no_gpl_cli_archive.py")
ARCHIVE_NAME = "xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip"
STAGED = f"{ARCHIVE_NAME[:-4]}/"

CLEAN_EXE = b"MZ\x90\x00 xberg formats --format json; tesseract; pdf; docx; xlsx"
HEIF_EXE = CLEAN_EXE + b" libheif heif_image_create"
DE265_DLL = b"MZ\x90\x00 de265_ libde265 decode"
DEFAULT_FORMATS = ["pdf", "docx", "xlsx", "txt"]


def run_gate(
    directory: Path, files: dict[str, bytes], formats: list[str], *required: str
) -> subprocess.CompletedProcess[str]:
    """Build an archive in ``directory`` and run the gate against it."""
    archive = directory / ARCHIVE_NAME
    with zipfile.ZipFile(archive, "w") as handle:
        for name, payload in files.items():
            handle.writestr(f"{STAGED}{name}", payload)
    formats_json = directory / "formats.json"
    entries = [{"extension": extension, "mime_type": f"application/{extension}"} for extension in formats]
    formats_json.write_text(json.dumps(entries), encoding="utf-8")
    command = [sys.executable, str(SCRIPT), "--archive", str(archive), "--formats-json", str(formats_json)]
    for extension in required:
        command.extend(["--require-format", extension])
    return subprocess.run(command, capture_output=True, text=True, check=False)


def case_clean_archive_passes() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(
            Path(tmp),
            {"xberg.exe": CLEAN_EXE, "LICENSE": b"MIT", "THIRD_PARTY_LICENSES.md": b"libheif?"},
            DEFAULT_FORMATS,
            "pdf",
            "docx",
            "xlsx",
        )
    assert result.returncode == 0, result.stderr
    assert "free of GPL/LGPL codecs" in result.stdout, result.stdout


def case_codec_symbol_in_executable_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {"xberg.exe": HEIF_EXE}, DEFAULT_FORMATS, "pdf")
    assert result.returncode == 1, result.stdout
    assert "xberg.exe contains libheif" in result.stderr, result.stderr


def case_codec_symbol_in_a_sibling_dll_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {"xberg.exe": CLEAN_EXE, "de265.dll": DE265_DLL}, DEFAULT_FORMATS, "pdf")
    assert result.returncode == 1, result.stdout
    assert "de265.dll contains libde265" in result.stderr, result.stderr


def case_text_files_are_not_scanned() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(
            Path(tmp),
            {"xberg.exe": CLEAN_EXE, "THIRD_PARTY_LICENSES.md": b"libheif libde265 x265"},
            DEFAULT_FORMATS,
            "pdf",
        )
    assert result.returncode == 0, result.stderr


def case_lost_office_formats_fail() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {"xberg.exe": CLEAN_EXE}, ["pdf", "txt"], "pdf", "docx", "xlsx")
    assert result.returncode == 1, result.stdout
    assert "does not report docx, xlsx" in result.stderr, result.stderr


def case_missing_archive_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        formats_json = directory / "formats.json"
        formats_json.write_text(json.dumps(DEFAULT_FORMATS), encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--archive",
                str(directory / "absent.zip"),
                "--formats-json",
                str(formats_json),
                "--require-format",
                "pdf",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    assert result.returncode == 1, result.stdout
    assert "archive not found" in result.stderr, result.stderr


def case_archive_without_executable_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_gate(Path(tmp), {"LICENSE": b"MIT"}, DEFAULT_FORMATS, "pdf")
    assert result.returncode == 1, result.stdout
    assert "holds no executable" in result.stderr, result.stderr


CASES = (
    case_clean_archive_passes,
    case_codec_symbol_in_executable_fails,
    case_codec_symbol_in_a_sibling_dll_fails,
    case_text_files_are_not_scanned,
    case_lost_office_formats_fail,
    case_missing_archive_fails,
    case_archive_without_executable_fails,
)


def main() -> int:
    for case in CASES:
        case()
        print(f"ok  {case.__name__}")
    print(f"{len(CASES)} no-GPL archive gate cases passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
