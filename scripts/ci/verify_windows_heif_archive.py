#!/usr/bin/env python3
"""Fail when the Windows CLI ZIP does not meet the LGPL-3.0 codec obligations.

libheif and libde265 are LGPL-3.0. Statically linking them into ``xberg.exe``
would put the archive under section 4(d)(0) of the licence: it would have to
carry the Minimal Corresponding Source and relinkable application objects, or a
written offer for them. The Windows CLI avoids that by linking shared libraries:
``xberg.exe`` imports ``heif.dll``, and the codec DLLs travel beside it as
separate, replaceable files.

Section 4(b) still applies to that combined work, so the archive must carry the
GNU GPL and GNU LGPL texts and a prominent notice. This gate checks the shipped
bytes, not the build script's intent: the executable's import table, the codec
DLLs and their dependency closure, the two licence texts, the upstream
per-port copyright files and ``THIRD_PARTY_LICENSES.md``. A return to static
linking therefore fails here instead of shipping silently.
"""

from __future__ import annotations

import argparse
import sys
import zipfile
from pathlib import Path

import windows_wheel_closure as closure

# vcpkg's dynamic port names libheif's DLL `heif.dll` (the CMake target is `heif`;
# upstream only sets `PREFIX lib` on Windows for libde265). Accept `libheif.dll` too so
# a future upstream prefix change does not turn a compliant archive red.
LIBHEIF_DLL_NAMES = ("heif.dll", "libheif.dll")
CODEC_DLLS = ("libde265.dll",)

# Every text is identified by strings that only the real licence carries, so an
# empty or placeholder file cannot satisfy the gate.
LICENSE_TEXTS = {
    "LGPL-3.0.txt": (b"GNU LESSER GENERAL PUBLIC LICENSE", b"Version 3"),
    "GPL-3.0.txt": (b"GNU GENERAL PUBLIC LICENSE", b"Version 3"),
}

CODEC_NOTICES = {
    "libheif-COPYING": (b"GNU LESSER GENERAL PUBLIC LICENSE",),
    "libde265-COPYING": (b"GNU LESSER GENERAL PUBLIC LICENSE",),
    "aom-COPYING": (b"Alliance for Open Media",),
}

PROJECT_NOTICE = "THIRD_PARTY_LICENSES.md"
PROJECT_NOTICE_MARKERS = (b"heif.dll", b"Windows")
STATIC_HINT = "section 4(d)(0)"


def file_members(names: list[str]) -> list[str]:
    """Return the archive's file members in name order."""
    return sorted(name for name in names if not name.endswith("/"))


def member_named(names: list[str], basename: str) -> str | None:
    """Return the first archive member whose basename matches, case-insensitively."""
    wanted = basename.casefold()
    return next((name for name in names if Path(name).name.casefold() == wanted), None)


def linkage_problems(archive: zipfile.ZipFile, names: list[str]) -> list[str]:
    """Return the reasons the executable does not dynamically link the codecs."""
    problems = []
    executables = [name for name in names if name.lower().endswith(".exe")]
    if not executables:
        return ["the archive holds no executable"]
    for executable in executables:
        imports = closure.PeImage(archive.read(executable)).imports()
        imported = next((candidate for candidate in LIBHEIF_DLL_NAMES if candidate in imports), None)
        if imported is None:
            problems.append(
                f"{executable} does not import {' or '.join(LIBHEIF_DLL_NAMES)}: a statically linked "
                f"libheif needs LGPL-3.0 {STATIC_HINT} relinkable object files or a written offer, "
                "and this archive carries neither"
            )
        elif member_named(names, imported) is None:
            problems.append(f"{executable} imports {imported} but the archive does not carry it")
    if not any(member_named(names, candidate) is not None for candidate in LIBHEIF_DLL_NAMES):
        problems.append(f"the archive does not carry {' or '.join(LIBHEIF_DLL_NAMES)} as a separate replaceable file")
    for dll in CODEC_DLLS:
        if member_named(names, dll) is None:
            problems.append(f"the archive does not carry {dll} as a separate replaceable file")
    return problems


def notice_problems(archive: zipfile.ZipFile, names: list[str]) -> list[str]:
    """Return the required notices that are missing or do not hold their license text."""
    problems = []
    expected = {**LICENSE_TEXTS, **CODEC_NOTICES}
    for basename, markers in expected.items():
        member = member_named(names, basename)
        if member is None:
            problems.append(f"the archive is missing the {basename} license text")
            continue
        payload = archive.read(member)
        missing = [marker.decode("ascii", "replace") for marker in markers if marker not in payload]
        if missing:
            problems.append(f"{basename} does not contain {', '.join(missing)}")
    notice = member_named(names, PROJECT_NOTICE)
    if notice is None:
        problems.append(f"the archive is missing {PROJECT_NOTICE}")
    else:
        payload = archive.read(notice)
        missing = [marker.decode("ascii", "replace") for marker in PROJECT_NOTICE_MARKERS if marker not in payload]
        if missing:
            problems.append(f"{PROJECT_NOTICE} does not document {', '.join(missing)}")
    return problems


def verify(archive_path: Path) -> list[str]:
    """Return every reason the archive fails the Windows LGPL codec contract."""
    if not archive_path.is_file():
        return [f"archive not found: {archive_path}"]
    with zipfile.ZipFile(archive_path) as archive:
        names = file_members(archive.namelist())
        problems = linkage_problems(archive, names)
        problems.extend(notice_problems(archive, names))
    try:
        files = closure._raw_artifact_files(archive_path)
        native = closure.roots(files, "xberg.exe")
        closure._audit_graph(
            files,
            "xberg.exe",
            "onnxruntime.dll",
            "directml.dll",
            closure.raw_external_policy(files, native, None, None),
        )
    except ValueError as error:
        problems.append(f"the codec DLL dependency closure is incomplete: {error}")
    return problems


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path, help="CLI .zip to audit")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    problems = verify(args.archive)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print(f"{args.archive.name} links libheif dynamically and ships the LGPL-3.0 notices")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
