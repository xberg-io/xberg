import hashlib
import os
import subprocess
import sys
import tarfile
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parent / "verify-php-macos-extension.sh"
STAGE = Path(__file__).parent / "stage-php-macos-notices.sh"
SOURCES = "".join(
    f"{name} 1.0\t{'0' * 64}\thttps://example.invalid/{name}.tar.gz\n" for name in ("libheif", "libde265", "libaom")
)
HOMEBREW_HEIF = "/opt/homebrew/opt/libheif/lib/libheif.1.dylib"

pytestmark = pytest.mark.skipif(sys.platform != "darwin", reason="needs otool and clang")


def build_extension(directory: Path, import_name: str | None, kind: str = "-dynamiclib", target: str = "15.0") -> Path:
    """Compile a real Mach-O xberg.so, optionally importing a dylib by install name."""
    source = directory / "ext.c"
    source.write_text("int ext(void) { return 0; }\n")
    extension = directory / "xberg.so"
    command = ["clang", kind, f"-mmacosx-version-min={target}", "-o", str(extension), str(source)]
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


def notice_files(directory: Path, sources: str = SOURCES) -> list[Path]:
    """Write the notices the staging step ships beside xberg.so, with a matching checksum list."""
    folder = directory / "notices"
    folder.mkdir()
    members = {
        "sources.tsv": sources,
        "libheif-1.0-COPYING": "lgpl\n",
        "LICENSE": "mit\n",
        "THIRD_PARTY_LICENSES.md": "tpl\n",
    }
    paths = []
    for name, text in members.items():
        (folder / name).write_text(text)
        paths.append(folder / name)
    rows = "".join(
        f"{hashlib.sha256((folder / name).read_bytes()).hexdigest()}  {name}\n"
        for name in ("sources.tsv", "libheif-1.0-COPYING")
    )
    (folder / "notices.sha256").write_text(rows)
    return [*paths, folder / "notices.sha256"]


def pack(directory: Path, members: list[Path]) -> Path:
    archive = directory / "php_xberg.tgz"
    with tarfile.open(archive, "w:gz") as tar:
        for member in members:
            tar.add(member, arcname=member.name)
    return archive


def archive_of(directory: Path, *members: Path, notices: bool = True) -> Path:
    return pack(directory, [*members, *(notice_files(directory) if notices else [])])


def run(archive: Path, floor: str = "15.0") -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "MACOSX_DEPLOYMENT_TARGET": floor}
    return subprocess.run([str(SCRIPT), str(archive)], capture_output=True, text=True, check=False, env=env)


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


def test_codec_import_under_a_system_path_is_rejected(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, "/usr/lib/libde265.0.dylib")))
    assert result.returncode == 1
    assert "linked statically" in result.stderr


def test_extension_above_the_deployment_floor_is_rejected(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, None, target="26.0")))
    assert result.returncode == 1
    assert "above the 15.0 floor" in result.stderr


def test_archive_without_notices_is_rejected(tmp_path: Path) -> None:
    result = run(archive_of(tmp_path, build_extension(tmp_path, None), notices=False))
    assert result.returncode == 1
    assert "does not ship LICENSE" in result.stderr


def test_notice_with_a_wrong_checksum_is_rejected(tmp_path: Path) -> None:
    extension = build_extension(tmp_path, None)
    notices = notice_files(tmp_path)
    notices[1].write_text("tampered\n")
    result = run(pack(tmp_path, [extension, *notices]))
    assert result.returncode == 1
    assert "notices.sha256 does not match" in result.stderr


def test_x265_in_the_source_inventory_is_rejected(tmp_path: Path) -> None:
    extension = build_extension(tmp_path, None)
    notices = notice_files(tmp_path, SOURCES + f"x265 1.0\t{'0' * 64}\thttps://example.invalid/x265\n")
    result = run(pack(tmp_path, [extension, *notices]))
    assert result.returncode == 1
    assert "exactly libheif, libde265 and libaom" in result.stderr


def test_staging_adds_notices_that_the_gate_accepts(tmp_path: Path) -> None:
    extension = build_extension(tmp_path, None)
    prefix = tmp_path / "prefix"
    inventory = prefix / "share" / "xberg-heif-notices"
    inventory.mkdir(parents=True)
    for source in notice_files(tmp_path):
        if source.name not in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
            (inventory / source.name).write_bytes(source.read_bytes())
    for name in range(10):
        (inventory / f"extra-{name}").write_text("x\n")
    project = tmp_path / "project"
    project.mkdir()
    (project / "LICENSE").write_text("mit\n")
    (project / "THIRD_PARTY_LICENSES.md").write_text("tpl\n")
    archive = tmp_path / "bare.tgz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(extension, arcname="xberg.so")
    staged = subprocess.run(
        [str(STAGE), str(archive), str(prefix), str(project)], capture_output=True, text=True, check=False
    )
    assert staged.returncode == 0, staged.stderr
    sidecar = archive.with_name("bare.tgz.sha256").read_text()
    assert sidecar.split()[0] == hashlib.sha256(archive.read_bytes()).hexdigest()
    result = run(archive)
    assert result.returncode == 0, result.stderr


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
