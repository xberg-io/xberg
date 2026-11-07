import subprocess
import sys
import tarfile
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parent / "verify-php-macos-extension.sh"
HOMEBREW_HEIF = "/opt/homebrew/opt/libheif/lib/libheif.1.dylib"

pytestmark = pytest.mark.skipif(sys.platform != "darwin", reason="needs otool and clang")


def build_extension(directory: Path, import_name: str | None, kind: str = "-dynamiclib") -> Path:
    """Compile a real Mach-O xberg.so, optionally importing a dylib by install name."""
    source = directory / "ext.c"
    source.write_text("int ext(void) { return 0; }\n")
    extension = directory / "xberg.so"
    command = ["clang", kind, "-o", str(extension), str(source)]
    if import_name is not None:
        dep_source = directory / "dep.c"
        dep_source.write_text("int dep(void) { return 1; }\n")
        dep = directory / "dep.dylib"
        subprocess.run(
            ["clang", "-dynamiclib", "-install_name", import_name, "-o", str(dep), str(dep_source)], check=True
        )
        command += [str(dep)]
    subprocess.run(command, check=True)
    return extension


def archive_of(directory: Path, *members: Path) -> Path:
    archive = directory / "php_xberg.tgz"
    with tarfile.open(archive, "w:gz") as tar:
        for member in members:
            tar.add(member, arcname=member.name)
    return archive


def run(archive: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run([str(SCRIPT), str(archive)], capture_output=True, text=True, check=False)


def test_homebrew_libheif_import_is_rejected(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, HOMEBREW_HEIF)))
    assert result.returncode == 1
    assert HOMEBREW_HEIF in result.stderr


def test_bundle_without_an_install_name_is_still_checked(tmp_path: Path) -> None:
    extension = build_extension(tmp_path, HOMEBREW_HEIF, kind="-bundle")
    assert (
        subprocess.run(["otool", "-D", str(extension)], capture_output=True, text=True, check=True).stdout.count("\n")
        == 1
    )
    result = run(archive_of(tmp_path, extension))
    assert result.returncode == 1
    assert HOMEBREW_HEIF in result.stderr


def test_system_only_imports_pass(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, None)))
    assert result.returncode == 0, result.stderr
    assert "checked 1 imports" in result.stderr or "checked 2 imports" in result.stderr


def test_system_path_import_passes(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, "/usr/lib/libz.1.dylib")))
    assert result.returncode == 0, result.stderr


def test_loader_path_sibling_is_rejected_because_pie_ships_one_file(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, "@loader_path/libheif.1.dylib")))
    assert result.returncode == 1
    assert "@loader_path/libheif.1.dylib" in result.stderr


def test_archive_without_extension_is_rejected(tmp_path: Path) -> None:
    other = tmp_path / "other.txt"
    other.write_text("x")
    result = run(archive_of(tmp_path, other))
    assert result.returncode == 1
    assert "exactly one xberg.so" in result.stderr
