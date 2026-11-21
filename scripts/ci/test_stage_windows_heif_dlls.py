#!/usr/bin/env python3
"""Contract tests for ``scripts/ci/stage_windows_heif_dlls.py``.

Each case builds a throwaway vcpkg bin directory from portable PE fixtures: the
three codec roots and their real imports are copied, while an unrelated DLL in
the same directory (``zlib1.dll``) is not.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

from test_windows_wheel_closure import pe

SCRIPT = Path(__file__).with_name("stage_windows_heif_dlls.py")


def dll(*imports: str) -> bytes:
    """Return a minimal x64 PE image with the given imports."""
    return bytes(pe(imports or ("KERNEL32.dll",)))


def tree(source: Path, heif: bytes | None = None, **extra: bytes) -> None:
    """Populate a vcpkg bin directory with the codec roots and extra DLLs."""
    source.mkdir(parents=True, exist_ok=True)
    if heif is not None:
        (source / "heif.dll").write_bytes(heif)
    (source / "libde265.dll").write_bytes(dll("KERNEL32.dll"))
    (source / "aom.dll").write_bytes(dll("KERNEL32.dll"))
    for name, payload in extra.items():
        (source / name).write_bytes(payload)


def run_stage(directory: Path) -> tuple[subprocess.CompletedProcess[str], Path]:
    """Stage from ``directory/source`` into ``directory/stage``."""
    destination = directory / "stage"
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--source",
            str(directory / "source"),
            "--destination",
            str(destination),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return result, destination


def case_copies_roots_and_their_imports_only() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        tree(
            directory / "source",
            heif=dll("libde265.dll", "aom.dll", "MSVCP140.dll", "KERNEL32.dll"),
            **{"zlib1.dll": dll("KERNEL32.dll"), "libsharpyuv.dll": dll("KERNEL32.dll")},
        )
        result, destination = run_stage(directory)
        assert result.returncode == 0, result.stderr
        assert sorted(path.name for path in destination.iterdir()) == ["aom.dll", "heif.dll", "libde265.dll"]


def case_copies_a_transitive_codec_dependency() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        tree(
            directory / "source",
            heif=dll("libde265.dll", "aom.dll", "libsharpyuv.dll", "KERNEL32.dll"),
            **{"libsharpyuv.dll": dll("KERNEL32.dll")},
        )
        result, destination = run_stage(directory)
        assert result.returncode == 0, result.stderr
        assert "libsharpyuv.dll" in {path.name for path in destination.iterdir()}


def case_libheif_prefixed_root_is_accepted() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        (directory / "source").mkdir(parents=True)
        (directory / "source" / "libheif.dll").write_bytes(dll("libde265.dll", "aom.dll", "KERNEL32.dll"))
        (directory / "source" / "libde265.dll").write_bytes(dll("KERNEL32.dll"))
        (directory / "source" / "aom.dll").write_bytes(dll("KERNEL32.dll"))
        result, destination = run_stage(directory)
        assert result.returncode == 0, result.stderr
        assert "libheif.dll" in {path.name for path in destination.iterdir()}


def case_missing_root_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        tree(directory / "source", heif=dll("libde265.dll", "aom.dll", "KERNEL32.dll"))
        (directory / "source" / "aom.dll").unlink()
        result, _ = run_stage(directory)
        assert result.returncode == 1, result.stdout
        assert "missing codec DLLs" in result.stderr, result.stderr


def case_missing_codec_import_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        tree(
            directory / "source",
            heif=dll("libde265.dll", "aom.dll", "libsharpyuv.dll", "KERNEL32.dll"),
        )
        result, _ = run_stage(directory)
        assert result.returncode == 1, result.stdout
        assert "imports libsharpyuv.dll, which is not in" in result.stderr, result.stderr


def case_unreadable_codec_image_fails() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        tree(directory / "source", heif=b"not a PE image")
        result, _ = run_stage(directory)
        assert result.returncode == 1, result.stdout
        assert "heif.dll is not a readable PE image" in result.stderr, result.stderr


CASES = (
    case_copies_roots_and_their_imports_only,
    case_copies_a_transitive_codec_dependency,
    case_libheif_prefixed_root_is_accepted,
    case_missing_root_fails,
    case_missing_codec_import_fails,
    case_unreadable_codec_image_fails,
)


def main() -> int:
    for case in CASES:
        case()
        print(f"ok  {case.__name__}")
    print(f"{len(CASES)} Windows HEIF DLL staging cases passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
