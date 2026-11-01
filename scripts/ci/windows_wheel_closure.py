"""Audit and stage wheel-local Windows PE dependencies without host DLL lookup."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import struct
import subprocess
import zipfile
from pathlib import Path, PurePosixPath

SYSTEM_DLLS = frozenset(
    [
        "advapi32",
        "bcrypt",
        "bcryptprimitives",
        "cfgmgr32",
        "combase",
        "comctl32",
        "comdlg32",
        "crypt32",
        "cryptbase",
        "dbghelp",
        "dnsapi",
        "dwmapi",
        "dxgi",
        "gdi32",
        "imm32",
        "iphlpapi",
        "kernel32",
        "kernelbase",
        "mpr",
        "msimg32",
        "mswsock",
        "ncrypt",
        "netapi32",
        "normaliz",
        "ntdll",
        "msvcrt",
        "ucrtbase",
        "pdh",
        "ole32",
        "oleaut32",
        "powrprof",
        "propsys",
        "psapi",
        "rpcrt4",
        "secur32",
        "setupapi",
        "shell32",
        "shlwapi",
        "user32",
        "userenv",
        "usp10",
        "uxtheme",
        "version",
        "winhttp",
        "wininet",
        "winmm",
        "winspool",
        "wintrust",
        "ws2_32",
        "wtsapi32",
    ]
)
MSVC_PREFIXES = ("msvcp", "vcruntime", "concrt", "vcomp")
VS2022_EDITIONS = ("Community", "Professional", "Enterprise")
REDIST_TERMS = "https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution#visual-c-runtime-files"
RUNTIME_TERMS = "https://visualstudio.microsoft.com/license-terms/vs2022-cruntime/"


class PeImage:
    def __init__(self, data: bytes) -> None:
        self.data = data
        if data[:2] != b"MZ":
            raise ValueError("invalid PE DOS header")
        offset = self.unpack("I", 0x3C)[0]
        if data[offset : offset + 4] != b"PE\0\0":
            raise ValueError("invalid PE signature")
        self.machine, sections = self.unpack("HH", offset + 4)
        optional = offset + 24
        magic = self.unpack("H", optional)[0]
        if magic not in (0x10B, 0x20B):
            raise ValueError("unsupported PE optional header")
        self.directory = optional + (112 if magic == 0x20B else 96)
        self.directory_count = self.unpack("I", self.directory - 4)[0]
        self.image_base = self.unpack("Q" if magic == 0x20B else "I", optional + (24 if magic == 0x20B else 28))[0]
        size = self.unpack("H", offset + 20)[0]
        if self.directory + 8 * min(self.directory_count, 16) > optional + size:
            raise ValueError("truncated PE directories")
        self.sections = [self.unpack("IIII", optional + size + i * 40 + 8) for i in range(sections)]

    def unpack(self, format_: str, offset: int) -> tuple[int, ...]:
        if offset < 0 or offset + struct.calcsize("<" + format_) > len(self.data):
            raise ValueError("truncated PE data")
        return struct.unpack_from("<" + format_, self.data, offset)

    def offset(self, rva: int) -> int:
        for virtual_size, address, raw_size, pointer in self.sections:
            if address <= rva < address + max(virtual_size, raw_size):
                delta = rva - address
                if delta < raw_size and pointer + delta < len(self.data):
                    return pointer + delta
        raise ValueError("invalid PE RVA")

    def name(self, rva: int) -> str:
        offset = self.offset(rva)
        end = self.data.find(b"\0", offset, min(offset + 512, len(self.data)))
        if end < 0:
            raise ValueError("unterminated PE DLL name")
        name = self.data[offset:end].decode("ascii")
        if not name or "/" in name or "\\" in name or ":" in name:
            raise ValueError("invalid PE DLL name")
        return name.lower()

    def imports(self) -> list[str]:
        result = []
        for index, words in [(1, 5), (13, 8)]:
            if index >= self.directory_count:
                continue
            rva, size = self.unpack("II", self.directory + index * 8)
            if not rva:
                continue
            if size < words * 4:
                raise ValueError("invalid PE import directory")
            for number in range(min(size // (words * 4), 65536)):
                row = self.unpack("I" * words, self.offset(rva + number * words * 4))
                if not any(row):
                    break
                target = row[3] if index == 1 else row[1] - (0 if row[0] & 1 else self.image_base)
                result.append(self.name(target))
            else:
                raise ValueError("unterminated PE import directory")
        return result


def external(name: str) -> bool:
    return (
        name.startswith(("api-ms-win-", "ext-ms-win-"))
        or name.removesuffix(".dll") in SYSTEM_DLLS
        or re.fullmatch(r"python3(?:\d+)?\.dll", name) is not None
    )


def artifact_files(artifact: Path) -> dict[str, tuple[str, bytes]]:
    if artifact.is_dir():
        entries = (
            (str(path.relative_to(artifact)).replace(os.sep, "/"), path.read_bytes())
            for path in artifact.rglob("*")
            if path.is_file() and path.suffix.lower() in (".pyd", ".dll")
        )
    else:
        with zipfile.ZipFile(artifact) as archive:
            entries = [
                (entry.filename, archive.read(entry))
                for entry in archive.infolist()
                if not entry.is_dir() and entry.filename.lower().endswith((".pyd", ".dll"))
            ]
    result = {}
    for name, data in entries:
        if name.lower() in result:
            raise ValueError(f"case-insensitive PE path collision: {name}")
        result[name.lower()] = (name, data)
    return result


def roots(files: dict[str, tuple[str, bytes]], native_glob: str) -> list[str]:
    found = [key for key in files if fnmatch.fnmatchcase(PurePosixPath(key).name, native_glob.lower())]
    if not found:
        raise ValueError(f"no native file matching {native_glob}")
    return found


def audit(
    files: dict[str, tuple[str, bytes]],
    native_glob: str,
    required: str = "onnxruntime.dll",
    forbidden: str = "directml.dll",
) -> dict[str, int]:
    native = roots(files, native_glob)
    machines = {PurePosixPath(key).parent: PeImage(files[key][1]).machine for key in native}
    for directory in machines:
        if str(directory / required.lower()) not in files:
            raise ValueError(f"missing {required} alongside native module")
    inspected = 0
    edges = 0
    for key, (name, data) in files.items():
        directory = PurePosixPath(key).parent
        if directory not in machines:
            raise ValueError(f"PE file outside native search directory: {name}")
        image = PeImage(data)
        if image.machine != machines[directory]:
            raise ValueError(f"PE architecture mismatch: {name}")
        inspected += 1
        for dependency in image.imports():
            edges += 1
            if dependency == forbidden.lower():
                raise ValueError(f"{name} imports forbidden {dependency}")
            if not external(dependency) and str(directory / dependency) not in files:
                raise ValueError(f"{name} missing wheel-local dependency {dependency}")
    return {"files": inspected, "edges": edges, "roots": len(native)}


def crt_directory(machine: int, explicit: str | None) -> Path:
    architecture = {0x8664: "x64", 0xAA64: "arm64", 0x14C: "x86"}.get(machine)
    if architecture is None:
        raise ValueError(f"unsupported PE architecture {machine}")
    source = explicit or os.environ.get("VCToolsRedistDir")  # noqa: SIM112 - Visual Studio documents this exact spelling.
    if not source:
        installed = Path(os.environ.get("PROGRAMFILES(X86)", "C:/Program Files (x86)"))
        tool = installed / "Microsoft Visual Studio/Installer/vswhere.exe"
        result = subprocess.run(
            [
                str(tool),
                "-latest",
                "-version",
                "[17.0,18.0)",
                "-products",
                *(f"Microsoft.VisualStudio.Product.{edition}" for edition in VS2022_EDITIONS),
                "-property",
                "installationPath",
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        source = str(Path(result.stdout.strip()) / "VC/Redist/MSVC")
    root = Path(source).resolve(strict=True)
    candidates = (
        [root] if root.name == "Microsoft.VC143.CRT" else list(root.glob(f"**/{architecture}/Microsoft.VC143.CRT"))
    )
    valid = [
        path
        for path in candidates
        if path.parent.name == architecture
        and [part.casefold() for part in path.parts[-9:-7]] == ["microsoft visual studio", "2022"]
        and path.parts[-7] in VS2022_EDITIONS
        and [part.lower() for part in path.parts[-6:-3]] == ["vc", "redist", "msvc"]
        and not any("debug" in part.lower() for part in path.parts)
    ]
    if not valid:
        raise ValueError("licensed Visual Studio 2022 release CRT redist directory not found")
    return max(valid, key=lambda path: tuple(int(value) for value in re.findall(r"\d+", str(path))))


def runtime_file(crt: Path, dependency: str, machine: int) -> tuple[Path, bytes]:
    matches = [path for path in crt.iterdir() if path.name.lower() == dependency and path.is_file()]
    if len(matches) != 1 or not matches[0].resolve().is_relative_to(crt):
        raise ValueError(f"release CRT dependency unavailable: {dependency}")
    path = matches[0]
    data = path.read_bytes()
    if PeImage(data).machine != machine:
        raise ValueError(f"CRT architecture mismatch: {dependency}")
    return path, data


def stage(root: Path, native_glob: str, source: str | None) -> dict[str, list[str]]:
    files = artifact_files(root)
    roots(files, native_glob)
    queue = list(files)
    added = {}
    provenance = {}
    directories = {}
    while queue:
        key = queue.pop()
        image = PeImage(files[key][1])
        for dependency in image.imports():
            if not dependency.startswith(MSVC_PREFIXES):
                continue
            if image.machine not in directories:
                directories[image.machine] = crt_directory(image.machine, source)
            path, data = runtime_file(directories[image.machine], dependency, image.machine)
            destination = str(PurePosixPath(key).parent / dependency)
            name = files[destination][0] if destination in files else str(PurePosixPath(key).parent / path.name)
            provenance[destination] = {
                "file": name,
                "source": "/".join(path.parts[-7:]),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
            if destination in files:
                if files[destination][1] != data:
                    raise ValueError(f"CRT byte collision: {dependency}")
                continue
            files[destination] = (name, data)
            added[destination] = data
            queue.append(destination)
    audit(files, native_glob)
    return write_staged(root, files, added, [provenance[key] for key in sorted(provenance)])


def notice_plan(root: Path, provenance: list[dict[str, str]]) -> tuple[Path, bytes] | None:
    if not provenance:
        return None
    dist_info = list(root.glob("*.dist-info"))
    if len(dist_info) != 1:
        raise ValueError("expected one wheel dist-info for Microsoft notices")
    notice = dist_info[0] / "licenses/MICROSOFT_VC_RUNTIME.json"
    payload = {
        "component": "Microsoft Visual C++ Runtime",
        "copyright": "Microsoft Corporation",
        "distribution_authority": REDIST_TERMS,
        "runtime_use_terms": RUNTIME_TERMS,
        "notice": "Unmodified Microsoft files; not licensed under Xberg's MIT license. "
        "Use is subject to Microsoft's runtime terms; redistribution requires the "
        "applicable licensed Visual Studio terms. Preserve these notices.",
        "files": provenance,
    }
    data = (json.dumps(payload, indent=2) + "\n").encode()
    if notice.exists():
        if notice.read_bytes() != data:
            raise ValueError("Microsoft runtime notice collision; rebuild the wheel from clean inputs")
        return None
    return notice, data


def write_staged(
    root: Path,
    files: dict[str, tuple[str, bytes]],
    added: dict[str, bytes],
    provenance: list[dict[str, str]],
) -> dict[str, list[str]]:
    notice = notice_plan(root, provenance)
    paths = []
    for key, data in added.items():
        path = root / files[key][0]
        path.write_bytes(data)
        paths.append(str(path))
    if notice is not None:
        path, data = notice
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(data)
        paths.append(str(path))
    return {"added": paths}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["audit", "stage"])
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--native-glob", default="*.pyd")
    parser.add_argument("--crt-dir")
    parser.add_argument("--required", default="onnxruntime.dll")
    parser.add_argument("--forbidden", default="DirectML.dll")
    args = parser.parse_args()
    try:
        if args.mode == "stage":
            print(json.dumps(stage(args.artifact, args.native_glob, args.crt_dir)))
        else:
            result = audit(artifact_files(args.artifact), args.native_glob, args.required, args.forbidden)
            print(
                f"Verified {result['files']} PE files, {result['edges']} import edges, {result['roots']} native roots"
            )
    except (
        ValueError,
        OSError,
        UnicodeError,
        struct.error,
        subprocess.CalledProcessError,
        zipfile.BadZipFile,
    ) as error:
        parser.exit(1, f"Windows wheel closure: {error}\n")


if __name__ == "__main__":
    main()
