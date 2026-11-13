"""Validate Android JNI release inputs and the AAR's packaged libraries."""

import argparse
import shutil
import struct
import zipfile
from pathlib import Path
from typing import BinaryIO

MACHINES = {"arm64-v8a": 183, "x86_64": 62}
LIBRARY = "libxberg_jni.so"


def verify_elf(library: BinaryIO, size: int, abi: str, label: str) -> None:
    header = library.read(64)
    if len(header) != 64 or header[:7] != b"\x7fELF\x02\x01\x01":
        raise ValueError(f"{label}: expected a little-endian ELF64 header")
    kind, machine = struct.unpack_from("<HH", header, 16)
    if kind != 3 or machine != MACHINES[abi]:
        raise ValueError(
            f"{label}: expected {abi} shared library (ELF machine {MACHINES[abi]}), got {kind=}, {machine=}"
        )

    version = struct.unpack_from("<I", header, 20)[0]
    program_offset, section_offset = struct.unpack_from("<QQ", header, 32)
    header_size, program_size, program_count, section_size, section_count = struct.unpack_from("<5H", header, 52)
    if version != 1 or header_size != 64:
        raise ValueError(f"{label}: invalid ELF header size or version")
    if program_size != 56 or not program_count or not 64 <= program_offset <= size - program_size * program_count:
        raise ValueError(f"{label}: missing or truncated ELF program header table")
    if section_offset and (
        section_size != 64 or not section_count or section_offset + section_size * section_count > size
    ):
        raise ValueError(f"{label}: truncated or unsupported ELF section header table")

    library.seek(program_offset)
    executable_load = False
    for _ in range(program_count):
        program = library.read(program_size)
        if len(program) != program_size:
            raise ValueError(f"{label}: truncated ELF program header")
        kind, flags, offset, address, _, file_size, memory_size, alignment = struct.unpack("<II6Q", program)
        if file_size and offset + file_size > size:
            raise ValueError(f"{label}: truncated ELF segment")
        if kind != 1:  # PT_LOAD
            continue
        if file_size > memory_size or (
            alignment > 1 and (alignment & (alignment - 1) or offset % alignment != address % alignment)
        ):
            raise ValueError(f"{label}: invalid ELF load segment")
        executable_load |= bool(flags & 1 and file_size)
    if not executable_load:
        raise ValueError(f"{label}: no file-backed executable ELF load segment")


def stage(source: Path, destination: Path) -> None:
    inputs = [(abi, source / abi / LIBRARY) for abi in MACHINES]
    for abi, path in inputs:
        with path.open("rb") as library:
            verify_elf(library, path.stat().st_size, abi, str(path))
    for abi, path in inputs:
        target = destination / abi / LIBRARY
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)


def verify_aar(path: Path) -> None:
    with zipfile.ZipFile(path) as archive:
        for abi in MACHINES:
            name = f"jni/{abi}/{LIBRARY}"
            entries = [entry for entry in archive.infolist() if entry.filename == name]
            if len(entries) != 1:
                raise ValueError(f"{path}: expected exactly one {name}, found {len(entries)}")
            with archive.open(entries[0]) as library:
                verify_elf(library, entries[0].file_size, abi, f"{path}:{name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    staging = subcommands.add_parser("stage")
    staging.add_argument("source", type=Path)
    staging.add_argument("destination", type=Path)
    checking = subcommands.add_parser("verify-aar")
    checking.add_argument("aar", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "stage":
            stage(args.source, args.destination)
        else:
            verify_aar(args.aar)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        parser.exit(1, f"Android JNI guard: {error}\n")


if __name__ == "__main__":
    main()
