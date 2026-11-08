import hashlib
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parent / "verify-php-macos-extension.sh"
STAGE = Path(__file__).parent / "stage-php-macos-notices.sh"
SMOKE = Path(__file__).parent / "smoke-php-macos-heif.sh"
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


CODEC_NOTICES = ("libheif-1.0-COPYING", "libde265-1.0-COPYING", "libaom-1.0-LICENSE", "GPL-3.0.txt")
PROJECT_NOTICES = ("LICENSE", "THIRD_PARTY_LICENSES.md")


def notice_files(
    directory: Path, sources: str = SOURCES, omit: str | None = None, unlisted: str | None = None
) -> list[Path]:
    """Write the full notice set the staging step ships beside xberg.so, with a matching checksum list.

    `omit` leaves one file out of the archive; `unlisted` ships it but leaves it out of notices.sha256.
    """
    folder = directory / "notices"
    folder.mkdir()
    members = {"sources.tsv": sources, **{name: f"{name}\n" for name in (*CODEC_NOTICES, *PROJECT_NOTICES)}}
    paths = []
    for name, text in members.items():
        if name == omit:
            continue
        (folder / name).write_text(text)
        paths.append(folder / name)
    rows = "".join(
        f"{hashlib.sha256((folder / name).read_bytes()).hexdigest()}  {name}\n"
        for name in ("sources.tsv", *CODEC_NOTICES)
        if name not in (omit, unlisted)
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
    next(path for path in notices if path.name == "libheif-1.0-COPYING").write_text("tampered\n")
    result = run(pack(tmp_path, [extension, *notices]))
    assert result.returncode == 1
    assert "notices.sha256 does not match" in result.stderr


def test_x265_in_the_source_inventory_is_rejected(tmp_path: Path) -> None:
    extension = build_extension(tmp_path, None)
    notices = notice_files(tmp_path, SOURCES + f"x265 1.0\t{'0' * 64}\thttps://example.invalid/x265\n")
    result = run(pack(tmp_path, [extension, *notices]))
    assert result.returncode == 1
    assert "exactly libheif, libde265 and libaom" in result.stderr


@pytest.mark.parametrize(
    "missing", ["libheif-1.0-COPYING", "libde265-1.0-COPYING", "libaom-1.0-LICENSE", "GPL-3.0.txt"]
)
def test_a_missing_codec_or_gpl_notice_is_rejected(tmp_path: Path, missing: str) -> None:
    extension = build_extension(tmp_path, None)
    result = run(pack(tmp_path, [extension, *notice_files(tmp_path, omit=missing)]))
    assert result.returncode == 1
    assert f"does not ship a {missing.replace('1.0', '*')} notice" in result.stderr


@pytest.mark.parametrize(
    "unlisted", ["libheif-1.0-COPYING", "libde265-1.0-COPYING", "libaom-1.0-LICENSE", "GPL-3.0.txt", "sources.tsv"]
)
def test_a_notice_missing_from_the_checksum_list_is_rejected(tmp_path: Path, unlisted: str) -> None:
    extension = build_extension(tmp_path, None)
    result = run(pack(tmp_path, [extension, *notice_files(tmp_path, unlisted=unlisted)]))
    assert result.returncode == 1
    assert f"notices.sha256 does not list {unlisted}" in result.stderr


def stage_fixture(tmp_path: Path) -> subprocess.CompletedProcess[str]:
    prefix = tmp_path / "prefix"
    inventory = prefix / "share" / "xberg-heif-notices"
    inventory.mkdir(parents=True)
    for source in notice_files(tmp_path):
        if source.name not in PROJECT_NOTICES:
            (inventory / source.name).write_bytes(source.read_bytes())
    project = tmp_path / "project"
    project.mkdir()
    for name in PROJECT_NOTICES:
        (project / name).write_text("project\n")
    archive = tmp_path / "bare.tgz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(build_extension(tmp_path, None), arcname="xberg.so")
    return subprocess.run(
        [str(STAGE), str(archive), str(prefix), str(project)], capture_output=True, text=True, check=False
    )


def test_staging_adds_notices_that_the_gate_accepts(tmp_path: Path) -> None:
    staged = stage_fixture(tmp_path)
    assert staged.returncode == 0, staged.stderr
    archive = tmp_path / "bare.tgz"
    sidecar = archive.with_name("bare.tgz.sha256").read_text()
    assert sidecar.split()[0] == hashlib.sha256(archive.read_bytes()).hexdigest()
    with tarfile.open(archive) as tar:
        assert len(tar.getnames()) == 1 + len(PROJECT_NOTICES) + 1 + 1 + len(CODEC_NOTICES)
    result = run(archive)
    assert result.returncode == 0, result.stderr


def test_staging_refuses_an_inventory_without_the_gpl_text(tmp_path: Path) -> None:
    prefix = tmp_path / "prefix"
    (prefix / "share" / "xberg-heif-notices").mkdir(parents=True)
    (prefix / "share" / "xberg-heif-notices" / "sources.tsv").write_text(SOURCES)
    archive = tmp_path / "bare.tgz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(build_extension(tmp_path, None), arcname="xberg.so")
    staged = subprocess.run(
        [str(STAGE), str(archive), str(prefix), str(tmp_path)], capture_output=True, text=True, check=False
    )
    assert staged.returncode == 1
    assert "no notices.sha256" in staged.stderr


@pytest.mark.skipif(shutil.which("php") is None, reason="needs php")
def test_smoke_rejects_an_extension_php_cannot_load(tmp_path: Path) -> None:
    result = subprocess.run(
        [str(SMOKE), str(archive_of(tmp_path, build_extension(tmp_path, None)))],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert "PHP did not load the archived xberg.so" in result.stderr
