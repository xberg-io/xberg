#!/usr/bin/env python3
"""Wrap the released CLI archives into per-platform npm packages plus the root manifest.

Reads `xberg-cli-<target>.tar.gz|zip` and `cli-SHA256SUMS` from --archives, verifies every archive
it uses against the checksum file, and writes under --out:

    <out>/<os>-<cpu>[-<libc>]/   one package per target that has an archive
    <out>/root/package.json      the root manifest with optionalDependencies pinned to --version

Each archive packs one top-level directory holding the binary and its native closure (vendored
shared libraries, the musl loader tree). The whole directory becomes the package's `bin/`, because
the binary finds those files relative to itself. Targets without an archive are skipped and
reported; the root launcher falls back to its download path on those platforms.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import tarfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

REPO_ROOT = Path(__file__).resolve().parents[2]
ROOT_MANIFEST = REPO_ROOT / "cli-proxy" / "npm" / "package.json"
ROOT_PACKAGE = "@xberg-io/xberg-cli"
BIN_NAME = "xberg"
SUMS_NAME = "cli-SHA256SUMS"
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$")
SUMS_LINE_RE = re.compile(r"([0-9a-fA-F]{64}) [ *]?(\S.*)")
MAX_FILE_BYTES = 512 * 1024 * 1024
MAX_TREE_BYTES = 1024 * 1024 * 1024
MAX_LINK_DEPTH = 8


class PackagingError(Exception):
    """A release archive is missing, unsafe or fails verification."""


@dataclass(frozen=True)
class Target:
    """One release target and the npm platform it maps to."""

    triple: str
    os: str
    cpu: str
    libc: str | None = None

    @property
    def suffix(self) -> str:
        """Directory and package-name suffix, e.g. linux-x64-musl."""
        return "-".join(part for part in (self.os, self.cpu, self.libc) if part)

    @property
    def package(self) -> str:
        """Scoped npm package name."""
        return f"{ROOT_PACKAGE}-{self.suffix}"

    @property
    def archive(self) -> str:
        """Release asset file name."""
        return f"xberg-cli-{self.triple}.{'zip' if self.os == 'win32' else 'tar.gz'}"

    @property
    def binary(self) -> str:
        """Executable file name inside the archive and the package."""
        return f"{BIN_NAME}.exe" if self.os == "win32" else BIN_NAME


# libc names follow the napi convention: glibc packages end in `-gnu`, musl in `-musl`, so the two
# Linux variants of one cpu have distinct names and the `libc` field makes npm install only one.
TARGETS = (
    Target("x86_64-unknown-linux-gnu", "linux", "x64", "gnu"),
    Target("aarch64-unknown-linux-gnu", "linux", "arm64", "gnu"),
    Target("x86_64-unknown-linux-musl", "linux", "x64", "musl"),
    Target("aarch64-unknown-linux-musl", "linux", "arm64", "musl"),
    Target("x86_64-apple-darwin", "darwin", "x64"),
    Target("aarch64-apple-darwin", "darwin", "arm64"),
    Target("x86_64-pc-windows-msvc", "win32", "x64"),
    Target("aarch64-pc-windows-msvc", "win32", "arm64"),
)
LIBC_FIELD = {"gnu": "glibc", "musl": "musl"}


def parse_checksums(path: Path) -> dict[str, str]:
    if not path.is_file():
        raise PackagingError(f"{SUMS_NAME} not found in {path.parent}")
    sums: dict[str, str] = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        match = SUMS_LINE_RE.fullmatch(line)
        if not match:
            raise PackagingError(f"{SUMS_NAME} line {number} is not in sha256sum format")
        digest = match.group(1).lower()
        if sums.setdefault(match.group(2), digest) != digest:
            raise PackagingError(f"{SUMS_NAME} lists {match.group(2)} twice with different digests")
    return sums


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _safe_parts(name: str) -> tuple[str, ...]:
    posix = PurePosixPath(name.replace("\\", "/"))
    if posix.is_absolute() or ".." in posix.parts or (posix.parts and re.fullmatch(r"[A-Za-z]:", posix.parts[0])):
        raise PackagingError(f"unsafe archive member {name!r}")
    return tuple(part for part in posix.parts if part != ".")


def _link_target(path: tuple[str, ...], target: str, archive: str) -> tuple[str, ...]:
    """Resolve a link target against the link's directory, refusing any escape from the archive."""
    if target.startswith("/") or re.match(r"[A-Za-z]:", target):
        raise PackagingError(f"{archive}: link {'/'.join(path)} has an absolute target")
    resolved = list(path[:-1])
    for part in PurePosixPath(target.replace("\\", "/")).parts:
        if part == "..":
            if not resolved:
                raise PackagingError(f"{archive}: link {'/'.join(path)} points outside the archive")
            resolved.pop()
        elif part != ".":
            resolved.append(part)
    return tuple(resolved)


def _resolve_links(files: dict[tuple[str, ...], bytes], links: dict[tuple[str, ...], str], archive: str) -> None:
    """Materialize symlinks and hardlinks as copies of the regular file they name inside the tree."""
    for path in links:
        current = path
        for _ in range(MAX_LINK_DEPTH):
            if current in links:
                current = _link_target(current, links[current], archive)
            elif current in files:
                files[path] = files[current]
                break
            else:
                raise PackagingError(f"{archive}: link {'/'.join(path)} does not resolve to a file in the archive")
        else:
            raise PackagingError(f"{archive}: link {'/'.join(path)} is nested too deeply")


def read_tree(archive: Path) -> dict[tuple[str, ...], tuple[bytes, bool]]:
    """Return every file of the archive's single top-level directory as {relative path: (bytes, executable)}."""
    raw: dict[tuple[str, ...], bytes] = {}
    modes: dict[tuple[str, ...], bool] = {}
    links: dict[tuple[str, ...], str] = {}
    total = 0

    def add(name: str, size: int, read: Callable[[], bytes], executable: bool) -> None:
        nonlocal total
        parts = _safe_parts(name)
        if size > MAX_FILE_BYTES:
            raise PackagingError(f"{archive.name}: {name} is implausibly large")
        total += size
        if total > MAX_TREE_BYTES:
            raise PackagingError(f"{archive.name}: contents are implausibly large")
        raw[parts] = read()
        modes[parts] = executable

    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zf:
            for info in zf.infolist():
                if not info.is_dir():
                    add(info.filename, info.file_size, lambda info=info: zf.read(info), True)
    else:
        with tarfile.open(archive, "r:gz") as tf:
            for member in tf:
                if member.isdir() or PurePosixPath(member.name).name.startswith("._"):
                    continue
                if member.issym() or member.islnk():
                    links[_safe_parts(member.name)] = member.linkname
                    modes[_safe_parts(member.name)] = bool(member.mode & 0o111)
                elif member.isfile():
                    handle = tf.extractfile(member)
                    if handle is None:
                        raise PackagingError(f"{archive.name}: cannot read {member.name}")
                    add(member.name, member.size, handle.read, bool(member.mode & 0o111))
                else:
                    raise PackagingError(f"{archive.name}: unsupported member type for {member.name!r}")
    _resolve_links(raw, links, archive.name)

    tops = {parts[0] for parts in raw if len(parts) > 1}
    if len(tops) != 1 or any(len(parts) == 1 for parts in raw):
        raise PackagingError(f"{archive.name}: expected exactly one top-level directory")
    (top,) = tops
    return {parts[1:]: (data, modes[parts]) for parts, data in raw.items() if parts[0] == top}


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def package_one(target: Target, archive: Path, version: str, base: dict, out: Path) -> str:
    tree = read_tree(archive)
    if (target.binary,) not in tree or not tree[(target.binary,)][0]:
        raise PackagingError(f"{archive.name} does not contain {target.binary}")
    pkg_dir = out / target.suffix
    bin_dir = pkg_dir / "bin"
    for parts, (data, executable) in sorted(tree.items()):
        dest = bin_dir.joinpath(*parts)
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        dest.chmod(0o755 if executable and target.os != "win32" else 0o644)
    files = ["bin", "README.md"]
    if (license_entry := tree.get(("LICENSE",))) is not None:
        (pkg_dir / "LICENSE").write_bytes(license_entry[0])
        files.append("LICENSE")
    (pkg_dir / "README.md").write_text(
        f"# {target.package}\n\nPrebuilt {BIN_NAME} binary for {target.os} {target.cpu}"
        f"{' (' + target.libc + ')' if target.libc else ''}. "
        f"It is installed automatically by `{ROOT_PACKAGE}`; install that instead.\n",
        encoding="utf-8",
    )
    manifest = {
        "name": target.package,
        "version": version,
        "description": f"Prebuilt {BIN_NAME} binary for {target.os} {target.cpu}",
        "license": base["license"],
        "author": base["author"],
        "repository": base["repository"],
        "os": [target.os],
        "cpu": [target.cpu],
        "files": files,
        "publishConfig": {"access": "public"},
    }
    if target.libc:
        manifest["libc"] = [LIBC_FIELD[target.libc]]
    write_json(pkg_dir / "package.json", manifest)
    return target.package


def package_all(
    archives: Path, version: str, out: Path, root_manifest: Path = ROOT_MANIFEST
) -> tuple[list[str], list[str]]:
    if not VERSION_RE.fullmatch(version):
        raise PackagingError(f"invalid version {version!r}: expected X.Y.Z or X.Y.Z-prerelease")
    sums = parse_checksums(archives / SUMS_NAME)
    base = json.loads(root_manifest.read_text(encoding="utf-8"))
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    built: list[str] = []
    skipped: list[str] = []
    for target in TARGETS:
        archive = archives / target.archive
        if not archive.is_file():
            skipped.append(target.package)
            continue
        expected = sums.get(target.archive)
        if expected is None:
            raise PackagingError(f"{target.archive} has no entry in {SUMS_NAME}; refusing to package it")
        actual = sha256_file(archive)
        if actual != expected:
            raise PackagingError(f"checksum mismatch for {target.archive} (expected {expected}, got {actual})")
        built.append(package_one(target, archive, version, base, out))
    if not built:
        raise PackagingError(f"no CLI archives found in {archives}; nothing to package")

    root = dict(base, version=version)
    root["optionalDependencies"] = dict.fromkeys(sorted(built), version)
    root_dir = out / "root"
    root_dir.mkdir()
    write_json(root_dir / "package.json", root)
    return built, skipped


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--archives", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        built, skipped = package_all(args.archives, args.version, args.out)
    except (PackagingError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"package_cli_npm: {error}", file=sys.stderr)
        return 1
    for name in built:
        print(f"built   {name}@{args.version}")
    for name in skipped:
        print(f"skipped {name} (no release archive)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
