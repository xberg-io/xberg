#!/usr/bin/env python3
"""Stage the dynamic HEIF codec DLLs and their runtime closure beside the CLI.

Only the codec roots and the non-system DLLs those roots actually import are
copied. A blind ``*.dll`` copy from the vcpkg bin directory would also ship
unrelated libraries (for example ``zlib1.dll``, which this build does not load),
so the copy follows the PE import graph instead. The archive-level closure audit
then proves the staged set is complete.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import windows_wheel_closure as closure

# vcpkg's dynamic libheif port emits `heif.dll`; accept upstream's `libheif.dll`
# spelling in case the CMake target gains a Windows `lib` prefix.
LIBHEIF_ROOTS = ("heif.dll", "libheif.dll")
REQUIRED_ROOTS = ("libde265.dll", "aom.dll")


def codec_roots(source: Path) -> list[str]:
    """Return the codec root DLL names present in ``source``, requiring each one."""
    names = {path.name.casefold(): path.name for path in source.iterdir() if path.is_file()}
    libheif = [name for name in LIBHEIF_ROOTS if name in names]
    if len(libheif) != 1:
        raise ValueError(f"expected exactly one of {' or '.join(LIBHEIF_ROOTS)} in {source}")
    missing = [name for name in REQUIRED_ROOTS if name not in names]
    if missing:
        raise ValueError(f"missing codec DLLs in {source}: {', '.join(missing)}")
    return [names[libheif[0]], *(names[name] for name in REQUIRED_ROOTS)]


def stage(source: Path, destination: Path) -> list[str]:
    """Copy the codec roots and their non-system imports; return the copied names."""
    if not source.is_dir():
        raise ValueError(f"codec DLL source is not a directory: {source}")
    destination.mkdir(parents=True, exist_ok=True)
    copied: dict[str, str] = {}
    pending = []
    for name in codec_roots(source):
        payload = (source / name).read_bytes()
        (destination / name).write_bytes(payload)
        copied[name.casefold()] = name
        pending.append((name, payload))
    while pending:
        name, payload = pending.pop()
        try:
            image = closure.PeImage(payload)
        except (ValueError, struct.error) as error:
            raise ValueError(f"{name} is not a readable PE image: {error}") from error
        for dependency in image.imports():
            key = dependency.casefold()
            if key in copied or closure.external(dependency) or dependency.startswith(closure.MSVC_PREFIXES):
                continue
            candidate = source / dependency
            if not candidate.is_file():
                raise ValueError(f"{name} imports {dependency}, which is not in {source}")
            dependency_payload = candidate.read_bytes()
            if closure.PeImage(dependency_payload).machine != image.machine:
                raise ValueError(f"{dependency} architecture does not match {name}")
            (destination / dependency).write_bytes(dependency_payload)
            copied[key] = dependency
            pending.append((dependency, dependency_payload))
    return sorted(copied.values())


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path, help="vcpkg dynamic-triplet bin directory")
    parser.add_argument("--destination", required=True, type=Path, help="staged CLI directory")
    args = parser.parse_args(argv)
    try:
        names = stage(args.source, args.destination)
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f"Windows HEIF DLL staging: {error}\n")
    print(json.dumps({"staged": names}))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
