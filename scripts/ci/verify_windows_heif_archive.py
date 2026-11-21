#!/usr/bin/env python3
"""Fail when the Windows CLI ZIP does not meet the LGPL-3.0 codec obligations.

libheif and libde265 are LGPL-3.0. Statically linking them into ``xberg.exe``
would put the archive under section 4(d)(0) of the licence: it would have to
carry the Minimal Corresponding Source and relinkable application objects, or a
written offer for them. The Windows CLI avoids the combined-work clause by
linking shared libraries: ``xberg.exe`` imports ``heif.dll``, and the codec DLLs
travel beside it as separate, replaceable files.

Two further obligations still apply to those DLLs and are checked here. LGPL-3.0
section 4(b) requires the GNU GPL and GNU LGPL texts and a prominent notice, and
section 3 (through GPL-3.0 section 6) requires the Corresponding Source or a
written offer for it, because the vcpkg ports patch the upstream releases. The
archive must therefore carry ``GPL-SOURCE-OFFER.txt`` naming the exact versions,
the vcpkg baseline, the patch set and the build triplet.

This gate checks the shipped bytes, not the build script's intent: the
executable's import table, the codec DLLs and their dependency closure, the two
licence texts, the upstream per-port copyright files, the written offer and
``THIRD_PARTY_LICENSES.md``. A return to static linking, or a dropped offer,
fails here instead of shipping silently.
"""

from __future__ import annotations

import argparse
import struct
import sys
import zipfile
from pathlib import Path

import windows_wheel_closure as closure

# vcpkg's dynamic port names libheif's DLL `heif.dll` (the CMake target is `heif`;
# upstream only sets `PREFIX lib` on Windows for libde265). Accept `libheif.dll` too so
# a future upstream prefix change does not turn a compliant archive red.
LIBHEIF_DLL_NAMES = ("heif.dll", "libheif.dll")
CODEC_DLLS = ("libde265.dll", "aom.dll")

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
# `§4(d)(1)` and the §2 modification notice are the statements the document must
# carry, not just the word "Windows".
PROJECT_NOTICE_MARKERS = (b"heif.dll", "\u00a74(d)(1)".encode(), b"Modification notice")

SOURCE_OFFER = "GPL-SOURCE-OFFER.txt"
# The offer is only an offer if it names the licence path, the term, the contact,
# the exact codecs, the vcpkg baseline, the patches and the build triplet.
SOURCE_OFFER_PHRASES = (
    b"LGPL-3.0",
    b"Corresponding Source",
    b"three years",
    b"section 6(b)",
    b"contact@xberg.io",
    b"5363acb796e576328288fae60c25304e227e9839",
    b"libheif 1.23.7",
    b"libde265 1.1.3",
    b"libaom 3.15.2",
    b"cxx-linkage-pkgconfig.diff",
    b"find-modules.diff",
    b"gdk-pixbuf.patch",
    b"symbol-exports.diff",
    b"vvc-pkgconfig.patch",
    b"fix-linkage.patch",
    b"aom-rename-static.diff",
    b"x64-windows",
)
STATIC_HINT = "section 4(d)(0)"


def file_members(names: list[str]) -> list[str]:
    """Return the archive's file members in name order."""
    return sorted(name for name in names if not name.endswith("/"))


def member_named(names: list[str], basename: str) -> str | None:
    """Return the first archive member whose basename matches, case-insensitively."""
    wanted = basename.casefold()
    return next((name for name in names if Path(name).name.casefold() == wanted), None)


def imported_libraries(archive: zipfile.ZipFile, executable: str, problems: list[str]) -> list[str]:
    """Return one executable's imports, recording unreadable PE data as a problem."""
    try:
        return closure.PeImage(archive.read(executable)).imports()
    except (ValueError, struct.error, UnicodeError) as error:
        problems.append(f"{executable} is not a readable PE image: {error}")
        return []


def linkage_problems(archive: zipfile.ZipFile, names: list[str]) -> list[str]:
    """Return the reasons the executable does not dynamically link the codecs."""
    problems = []
    executables = [name for name in names if name.lower().endswith(".exe")]
    if not executables:
        return ["the archive holds no executable"]
    for executable in executables:
        imports = imported_libraries(archive, executable, problems)
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


def offer_problems(archive: zipfile.ZipFile, names: list[str]) -> list[str]:
    """Return why the written corresponding-source offer is missing or incomplete."""
    member = member_named(names, SOURCE_OFFER)
    if member is None:
        return [f"the archive is missing the {SOURCE_OFFER} corresponding-source offer"]
    payload = archive.read(member)
    missing = [phrase.decode("ascii", "replace") for phrase in SOURCE_OFFER_PHRASES if phrase not in payload]
    if missing:
        return [f"{SOURCE_OFFER} does not name {', '.join(missing)}"]
    return []


def verify(archive_path: Path) -> list[str]:
    """Return every reason the archive fails the Windows LGPL codec contract."""
    if not archive_path.is_file():
        return [f"archive not found: {archive_path}"]
    try:
        with zipfile.ZipFile(archive_path) as archive:
            names = file_members(archive.namelist())
            problems = linkage_problems(archive, names)
            problems.extend(notice_problems(archive, names))
            problems.extend(offer_problems(archive, names))
    except (zipfile.BadZipFile, OSError, ValueError) as error:
        return [f"{archive_path} is not a readable ZIP archive: {error}"]
    try:
        closure.audit_raw_archive(archive_path, "xberg.exe")
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
    print(f"{args.archive.name} links libheif dynamically and ships the LGPL-3.0 texts and source offer")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
