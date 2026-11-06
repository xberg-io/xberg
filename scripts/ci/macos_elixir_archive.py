#!/usr/bin/env python3
"""Stage and verify a flat macOS arm64 Rustler release archive."""

import argparse
import gzip
import hashlib
import os
import re
import struct
import subprocess
import tarfile
import tempfile
from pathlib import Path
from typing import BinaryIO

SCRIPTS = Path(__file__).resolve().parent
MAX_UNCOMPRESSED = 256 * 1024 * 1024
MAX_NOTICE = 128 * 1024
BLOCK = tarfile.BLOCKSIZE
ZERO_BLOCK = bytes(BLOCK)
MAX_PAX_MTIME = 128
MACH_HEADER = struct.Struct("<8I")
MH_MAGIC_64 = 0xFEEDFACF
CPU_TYPE_ARM64 = 0x0100000C
MH_DYLIB = 6
MAX_LOAD_COMMANDS = 512
MAX_LOAD_COMMAND_BYTES = 64 * 1024
CODECS = {"libheif", "libde265", "libaom", "libx265"}
ARCHIVE_NAME = re.compile(
    r"libxberg_nif-v[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.+-]+)?-nif-[0-9]+\.[0-9]+"
    r"-aarch64-apple-darwin\.so\.tar\.gz"
)
CODEC_NAME = re.compile(r"(?:libheif|libde265|libaom|libx265)(?:\.[0-9]+)+\.dylib")
LOAD_COMMANDS = {
    "LC_LOAD_DYLIB",
    "LC_LOAD_WEAK_DYLIB",
    "LC_REEXPORT_DYLIB",
    "LC_LOAD_UPWARD_DYLIB",
    "LC_LAZY_LOAD_DYLIB",
}
SOURCE_URL = {
    "libheif": "https://github.com/strukturag/libheif/releases/download/v{v}/libheif-{v}.tar.gz",
    "libde265": "https://github.com/strukturag/libde265/releases/download/v{v}/libde265-{v}.tar.gz",
    "libaom": "https://storage.googleapis.com/aom-releases/libaom-{v}.tar.gz",
    "x265": "https://github.com/Multicorewareinc/x265/releases/download/{v}/x265_{v}.tar.gz",
}
NOTICE_SUFFIXES = {
    "libheif": ("COPYING",),
    "libde265": ("COPYING", "AUTHORS", "libde265-COPYING"),
    "libaom": (
        "LICENSE",
        "PATENTS",
        "AUTHORS",
        "third_party-fastfeat-LICENSE",
        "third_party-libyuv-LICENSE",
        "third_party-vector-LICENSE",
    ),
    "x265": ("COPYING",),
}


def read_exact(stream: BinaryIO, size: int) -> bytes:
    data = stream.read(size)
    if len(data) != size:
        raise ValueError("truncated tar header, payload, or padding")
    return data


def require_terminator(stream: BinaryIO) -> None:
    tail = stream.read(tarfile.RECORDSIZE + 1)
    if not BLOCK <= len(tail) <= tarfile.RECORDSIZE or len(tail) % BLOCK or any(tail):
        raise ValueError("invalid tar terminator or data after terminator")


def member_mode(name: str) -> int:
    return 0o755 if name.endswith((".so", ".dylib")) else 0o644


def verify_member_metadata(member: tarfile.TarInfo) -> None:
    if (
        member.type != tarfile.REGTYPE
        or member.mode != member_mode(member.name)
        or any((member.uid, member.gid, member.uname, member.gname, member.mtime, member.devmajor, member.devminor))
    ):
        raise ValueError(f"unexpected metadata on archive member {member.name}")


def read_member(stream: BinaryIO, header: bytes, *, allow_mtime_pax: bool) -> tarfile.TarInfo:
    member = tarfile.TarInfo.frombuf(header, "ascii", "strict")
    if not allow_mtime_pax or member.type != tarfile.XHDTYPE or member.name != "././@PaxHeader":
        return member
    # Rustler's Python tar writer uses a local fractional mtime record. Ignore
    # only that record after validating the physical header and exact payload.
    if member.linkname or not 0 < member.size <= MAX_PAX_MTIME:
        raise ValueError("invalid raw Rustler PAX mtime header")
    data = read_exact(stream, member.size)
    match = re.fullmatch(rb"([1-9][0-9]*) mtime=-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?\n", data)
    if not match or int(match[1]) != len(data) or any(read_exact(stream, (-member.size) % BLOCK)):
        raise ValueError("unexpected raw Rustler PAX metadata; only a single mtime row is allowed")
    return tarfile.TarInfo.frombuf(read_exact(stream, BLOCK), "ascii", "strict")


def unpack(archive: Path, directory: Path, allowed: set[str], *, canonical_modes: bool = False) -> None:
    """Copy only named regular files, with no interpreted PAX/GNU extension records."""
    seen = set()
    remaining = MAX_UNCOMPRESSED
    if not archive.is_file() or archive.stat().st_size > MAX_UNCOMPRESSED:
        raise ValueError(f"archive missing or exceeds size limit: {archive}")
    with gzip.open(archive, "rb") as stream:
        while (header := read_exact(stream, BLOCK)) != ZERO_BLOCK:
            member = read_member(stream, header, allow_mtime_pax=not canonical_modes)
            name = member.name
            if member.type not in (tarfile.REGTYPE, tarfile.AREGTYPE) or member.linkname:
                raise ValueError(f"expected a regular tar member: {name!r}")
            if name not in allowed:
                raise ValueError(f"unexpected member in archive: {name!r}")
            if name in seen:
                raise ValueError(f"duplicate archive member: {name}")
            if member.size <= 0 or member.size > remaining:
                raise ValueError(f"member exceeds size limit or is empty: {name}")
            if canonical_modes:
                verify_member_metadata(member)
            remaining -= member.size
            seen.add(name)
            with (directory / name).open("xb") as target:
                size = member.size
                while size:
                    data = read_exact(stream, min(size, 1024 * 1024))
                    target.write(data)
                    size -= len(data)
            if any(read_exact(stream, (-member.size) % BLOCK)):
                raise ValueError(f"nonzero tar padding after {name}")
            (directory / name).chmod(member_mode(name))
        require_terminator(stream)
    if missing := allowed - seen:
        raise ValueError(f"missing archive members: {', '.join(sorted(missing))}")


def read_notice(path: Path) -> bytes:
    if path.is_symlink() or not path.is_file() or not 0 < path.stat().st_size <= MAX_NOTICE:
        raise ValueError(f"invalid source notice file: {path}")
    return path.read_bytes()


def source_versions(sources: bytes) -> dict[str, str]:
    releases = {}
    for row in sources.decode("ascii").splitlines():
        fields = row.split("\t")
        if len(fields) != 3:
            raise ValueError("invalid codec source provenance row")
        name_version, digest, url = fields
        name, _, version = name_version.partition(" ")
        if (
            name not in SOURCE_URL
            or name in releases
            or not re.fullmatch(r"[0-9]+(?:\.[0-9]+)+", version)
            or not re.fullmatch(r"[0-9a-f]{64}", digest)
            or url != SOURCE_URL[name].format(v=version)
        ):
            raise ValueError(f"invalid codec source provenance row: {name_version!r}")
        releases[name] = version
    if releases.keys() != SOURCE_URL.keys():
        raise ValueError("incomplete codec source provenance")
    return releases


def notice_names(releases: dict[str, str]) -> set[str]:
    return {"sources.tsv"} | {
        f"{name}{'_' if name == 'x265' else '-'}{releases[name]}-{suffix}"
        for name, suffixes in NOTICE_SUFFIXES.items()
        for suffix in suffixes
    }


def read_notices(directory: Path) -> dict[str, bytes]:
    manifest = read_notice(directory / "notices.sha256")
    sources = read_notice(directory / "sources.tsv")
    names = notice_names(source_versions(sources))
    if {path.name for path in directory.iterdir()} != names | {"notices.sha256"}:
        raise ValueError("source notice inventory does not match required upstream files")
    digests = {}
    for row in manifest.decode("ascii").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", row)
        if not match or match[2] in digests:
            raise ValueError("invalid source notice inventory row")
        digests[match[2]] = match[1]
    if digests.keys() != names:
        raise ValueError("source notice inventory does not cover the required upstream files")
    notices = {name: read_notice(directory / name) for name in names}
    for name, data in notices.items():
        if hashlib.sha256(data).hexdigest() != digests[name]:
            raise ValueError(f"source notice checksum mismatch: {name}")
    return {**notices, "notices.sha256": manifest}


def verify_source_receipt(prefix: Path, sources: bytes) -> None:
    marker = prefix / ".xberg-heif-encoder-profile"
    if marker.is_symlink() or not marker.is_file() or marker.stat().st_size:
        raise ValueError("missing or invalid source codec profile marker")
    versions = source_versions(sources)
    expected = (
        f"libheif={versions['libheif']} libde265={versions['libde265']} libaom={versions['libaom']} "
        f"target=11.0 arch=arm64 profile=elixir-encoders x265={versions['x265']}\n"
    )
    if read_notice(prefix / ".xberg-heif-versions") != expected.encode("ascii"):
        raise ValueError("source codec build receipt does not match the notice source versions")


def tool(*args: str) -> str:
    try:
        return subprocess.run(
            args, check=True, capture_output=True, text=True, env={**os.environ, "LC_ALL": "C"}
        ).stdout
    except subprocess.CalledProcessError as error:
        raise ValueError(f"{args[0]} failed for {args[-1]}: {error.stderr.strip()}") from error


def verify_macho_header(path: Path) -> None:
    with path.open("rb") as source:
        header = source.read(MACH_HEADER.size)
    if len(header) != MACH_HEADER.size:
        raise ValueError(f"truncated Mach-O header in {path.name}")
    magic, cpu, _, file_type, count, command_bytes, _, _ = MACH_HEADER.unpack(header)
    if (magic, cpu, file_type) != (MH_MAGIC_64, CPU_TYPE_ARM64, MH_DYLIB):
        raise ValueError(f"{path.name} must be a thin arm64 Mach-O dylib")
    if (
        not 0 < count <= MAX_LOAD_COMMANDS
        or not count * 8 <= command_bytes <= MAX_LOAD_COMMAND_BYTES
        or MACH_HEADER.size + command_bytes > path.stat().st_size
    ):
        raise ValueError(f"invalid Mach-O load command budget in {path.name}")


def verify_floor(path: Path) -> None:
    output = tool(
        "bash",
        "-c",
        'source "$1"; macho_minos_by_arch "$2" 1',
        "_",
        str(SCRIPTS.parent / "lib/macho.sh"),
        str(path),
    )
    if output.splitlines() != ["arm64 11.0"]:
        raise ValueError(f"{path.name} must declare exactly one arm64 slice at macOS 11.0: {output.strip()}")


def install_id(path: Path) -> str:
    rows = tool("otool", "-D", str(path)).splitlines()
    if len(rows) != 2 or rows[0] != f"{path}:" or not rows[1]:
        raise ValueError(f"expected one Mach-O dylib identity in {path.name}")
    return rows[1]


def macho_imports(path: Path, *, expected_id: str | None = None) -> list[str]:
    identity = install_id(path)
    if expected_id is not None and identity != expected_id:
        raise ValueError(f"unexpected relocated dylib identity in {path.name}: {identity}")
    commands = tool("otool", "-arch", "arm64", "-l", str(path))
    command_types = re.findall(r"(?m)^\s*cmd (LC_\w+)[ \t]*$", commands)
    search_commands = {"LC_RPATH", "LC_DYLD_ENVIRONMENT", "LC_LOAD_DYLINKER", "LC_ID_DYLINKER"}
    if search_commands.intersection(command_types):
        raise ValueError(f"unsupported loader search command in {path.name}")
    rows = tool("otool", "-L", str(path)).splitlines()
    names = []
    if not rows or rows[0] != f"{path}:":
        raise ValueError(f"invalid Mach-O import header in {path.name}")
    for row in rows[1:]:
        match = re.fullmatch(r"[ \t]+([^ \t].*?) \(compatibility version [^)]+\)", row)
        if not match:
            raise ValueError(f"invalid Mach-O import row in {path.name}: {row!r}")
        names.append(match[1])
    count = sum(command in LOAD_COMMANDS for command in command_types)
    if command_types.count("LC_ID_DYLIB") != 1 or not count or len(names) != count + 1 or names[0] != identity:
        raise ValueError(f"missing, ambiguous, or incomplete Mach-O imports in {path.name}")
    return names[1:]


def clean_absolute(path: str) -> bool:
    return path.startswith("/") and all(part and part not in (".", "..") for part in path[1:].split("/"))


def system_import(path: str) -> bool:
    return path.startswith(("/usr/lib/", "/System/Library/")) and clean_absolute(path)


def source_library(library_dir: Path, dependency: str) -> Path:
    path = Path(dependency)
    if not clean_absolute(dependency) or not CODEC_NAME.fullmatch(path.name):
        raise ValueError(f"unsupported source codec import: {dependency}")
    if path.parent.resolve() != library_dir.resolve() or path.resolve().parent != library_dir.resolve():
        raise ValueError(f"dependency outside the codec prefix: {dependency}")
    if not path.is_file():
        raise ValueError(f"missing source codec dependency: {dependency}")
    return path


def final_siblings(owner: str, imports: list[str], siblings: set[str]) -> set[str]:
    edges = set()
    for dependency in imports:
        if system_import(dependency):
            continue
        name = dependency.removeprefix("@loader_path/")
        if name not in siblings or dependency != f"@loader_path/{name}" or name == owner:
            raise ValueError(f"unsupported import in {owner}: {dependency}")
        edges.add(name)
    return edges


def verify_graph(nif: str, edges: dict[str, set[str]]) -> None:
    names = set(edges) - {nif}
    by_codec = {name.split(".")[0]: name for name in names if CODEC_NAME.fullmatch(name)}
    if len(names) != len(CODECS) or by_codec.keys() != CODECS:
        raise ValueError("incorrect codec graph: expected exactly four source codecs")
    heif = by_codec["libheif"]
    expected = {nif: {heif}, heif: names - {heif}}
    expected.update({name: set() for name in names - {heif}})
    if edges != expected:
        raise ValueError(f"incorrect codec graph: {edges}")


def source_graph(nif: Path, library_dir: Path, codecs: set[str]) -> None:
    pending = [nif]
    edges: dict[str, set[str]] = {}
    while pending:
        path = pending.pop()
        if path.name in edges:
            continue
        verify_macho_header(path)
        verify_floor(path)
        dependencies = [
            source_library(library_dir, dependency)
            for dependency in macho_imports(path)
            if not system_import(dependency)
        ]
        edges[path.name] = {dependency.name for dependency in dependencies}
        if not edges[path.name] <= codecs:
            raise ValueError(f"unrecognized source codec identity in {path.name}")
        pending.extend(dependencies)
    verify_graph(nif.name, edges)


def codec_filenames(library_dir: Path) -> set[str]:
    names = set()
    for codec in sorted(CODECS):
        path = library_dir / f"{codec}.dylib"
        if path.resolve().parent != library_dir.resolve():
            raise ValueError(f"codec alias points outside the codec prefix: {path}")
        verify_macho_header(path)
        dependency = source_library(library_dir, install_id(path))
        if dependency.name.split(".")[0] != codec or not path.samefile(dependency):
            raise ValueError(f"wrong source dylib identity for {codec}: {dependency.name}")
        names.add(dependency.name)
    return names


def verify_release(archive: Path, directory: Path, root: str, codecs: set[str], notices: dict[str, bytes]) -> None:
    unpack(archive, directory, {root, *codecs, *notices}, canonical_modes=True)
    for name, expected in notices.items():
        if read_notice(directory / name) != expected:
            raise ValueError(f"archive source notice bytes differ: {name}")
    edges = {}
    for name in sorted({root, *codecs}):
        path = directory / name
        verify_macho_header(path)
        verify_floor(path)
        tool("codesign", "--verify", "--strict", str(path))
        expected_id = f"@loader_path/{name}" if name in codecs else None
        edges[name] = final_siblings(name, macho_imports(path, expected_id=expected_id), codecs)
    verify_graph(root, edges)


def pack(archive: Path, directory: Path, members: set[str]) -> None:
    with (
        archive.open("xb") as target,
        gzip.GzipFile(fileobj=target, mode="wb", filename="", mtime=0) as gzip_stream,
        tarfile.open(fileobj=gzip_stream, mode="w|", format=tarfile.USTAR_FORMAT) as tar,
    ):
        for name in sorted(members):
            path = directory / name
            member = tarfile.TarInfo(name)
            member.mode = member_mode(name)
            member.size = path.stat().st_size
            with path.open("rb") as source:
                tar.addfile(member, source)


def checksum_row(archive: Path) -> str:
    if not archive.is_file() or archive.stat().st_size > MAX_UNCOMPRESSED:
        raise ValueError(f"archive missing or exceeds size limit: {archive}")
    with archive.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return f"{digest}  {archive.name}\n"


def verify_checksum(archive: Path, sidecar: Path) -> None:
    if read_notice(sidecar) != checksum_row(archive).encode("ascii"):
        raise ValueError(f"archive checksum mismatch: {archive.name}")


def stage_archive(
    archive: Path, output_dir: Path, library_dir: Path, root: str, *, codecs: set[str], notices: dict[str, bytes]
) -> None:
    with tempfile.TemporaryDirectory(prefix=".elixir-archive-", dir=output_dir) as temporary:
        work = Path(temporary)
        stage = work / "stage"
        stage.mkdir()
        unpack(archive, stage, {root})
        source_graph(stage / root, library_dir, codecs)
        tool("bash", str(SCRIPTS / "vendor-native-closure-macos.sh"), str(stage / root))
        for name, data in notices.items():
            (stage / name).write_bytes(data)
        members = {root, *codecs, *notices}
        if {path.name for path in stage.iterdir()} != members:
            raise ValueError("unexpected member produced by native closure relocation")
        staged_archive = work / archive.name
        pack(staged_archive, stage, members)
        audit = work / "audit"
        audit.mkdir()
        verify_release(staged_archive, audit, root, codecs, notices)
        sidecar = work / f"{archive.name}.sha256"
        sidecar.write_text(checksum_row(staged_archive), encoding="ascii")
        verify_checksum(staged_archive, sidecar)
        publish_local(staged_archive, sidecar, output_dir)


def publish_local(archive: Path, sidecar: Path, output_dir: Path) -> None:
    published_sidecar = output_dir / sidecar.name
    os.link(sidecar, published_sidecar)
    try:
        os.link(archive, output_dir / archive.name)
    except OSError:
        published_sidecar.unlink()
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("stage", "verify"))
    parser.add_argument("archive", type=Path)
    parser.add_argument(
        "--codec-prefix", type=Path, required=True, help="verified arm64 macOS 11.0 elixir-encoders source prefix"
    )
    parser.add_argument("--output-dir", type=Path, help="existing output directory for stage and its checksum")
    args = parser.parse_args()
    if (args.operation == "stage") != (args.output_dir is not None):
        parser.error("--output-dir is required only for stage")
    try:
        archive = args.archive.absolute()
        if not ARCHIVE_NAME.fullmatch(archive.name):
            raise ValueError(f"unexpected arm64 Rustler archive name: {archive.name}")
        root = archive.name.removesuffix(".tar.gz")
        library_dir = args.codec_prefix.absolute() / "lib"
        notices = read_notices(library_dir.parent / "share/xberg-heif-notices")
        verify_source_receipt(library_dir.parent, notices["sources.tsv"])
        codecs = codec_filenames(library_dir)
        if args.operation == "stage":
            stage_archive(
                archive, args.output_dir.resolve(strict=True), library_dir, root, codecs=codecs, notices=notices
            )
        else:
            verify_checksum(archive, archive.with_name(f"{archive.name}.sha256"))
            with tempfile.TemporaryDirectory(prefix="elixir-archive-audit-") as temporary:
                verify_release(archive, Path(temporary), root, codecs, notices)
        print(f"verified macOS arm64 Elixir NIF, four codecs, and {len(notices)} source inventory files")
    except (ValueError, OSError, EOFError, tarfile.TarError) as error:
        parser.exit(1, f"macos-elixir-archive: {error}\n")


if __name__ == "__main__":
    main()
