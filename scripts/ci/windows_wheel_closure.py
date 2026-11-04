"""Audit and stage wheel-local Windows PE dependencies without host DLL lookup."""

from __future__ import annotations

import argparse
import ctypes
import fnmatch
import hashlib
import json
import os
import re
import stat
import struct
import subprocess
import tarfile
import zipfile
from ctypes import wintypes
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator, Sequence

_WINDOWS_GET_FILE_ATTRIBUTES = None
if os.name == "nt":
    _WINDOWS_GET_FILE_ATTRIBUTES = ctypes.WinDLL("kernel32", use_last_error=True).GetFileAttributesW
    _WINDOWS_GET_FILE_ATTRIBUTES.argtypes = [wintypes.LPCWSTR]
    _WINDOWS_GET_FILE_ATTRIBUTES.restype = wintypes.DWORD

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
PHP_QUERY_TIMEOUT_SECONDS = 10
RAW_CRT_NOTICE = "MICROSOFT_VC_RUNTIME.json"


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


def _audit_graph(
    files: dict[str, tuple[str, bytes]],
    native_glob: str,
    required: str,
    forbidden: str,
    allow_external: Callable[[str, str], bool],
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
            if not allow_external(key, dependency) and str(directory / dependency) not in files:
                raise ValueError(f"{name} missing wheel-local dependency {dependency}")
    return {"files": inspected, "edges": edges, "roots": len(native)}


def audit(
    files: dict[str, tuple[str, bytes]],
    native_glob: str,
    required: str = "onnxruntime.dll",
    forbidden: str = "directml.dll",
) -> dict[str, int]:
    return _audit_graph(files, native_glob, required, forbidden, lambda _key, name: external(name))


def _raw_member_paths(members: Sequence[tuple[str, bool]]) -> list[str]:
    names = []
    seen = set()
    for name, linked in members:
        if linked:
            raise ValueError(f"artifact links are forbidden: {name}")
        path = PurePosixPath(name)
        if (
            not name
            or path.is_absolute()
            or ".." in path.parts
            or any(ord(c) < 32 or c in '\\:<>"|?*' for c in name)
            or any(part.endswith((".", " ")) for part in path.parts)
            or any(
                re.fullmatch(r"con|prn|aux|nul|(?:com|lpt)[1-9¹²³]", part.split(".")[0].rstrip(" "), re.IGNORECASE)
                for part in path.parts
            )
        ):
            raise ValueError(f"unsafe artifact member: {name}")
        normalized = str(path)
        if normalized.casefold() in seen:
            raise ValueError(f"duplicate artifact member: {name}")
        seen.add(normalized.casefold())
        names.append(normalized)
    return names


def _raw_member_selected(name: str, selected: set[str] | None) -> bool:
    return name.lower().endswith((".dll", ".exe", ".pyd")) if selected is None else name.lower() in selected


def _windows_file_attributes(path: Path) -> int:
    if _WINDOWS_GET_FILE_ATTRIBUTES is None:
        return 0
    attributes = _WINDOWS_GET_FILE_ATTRIBUTES(str(path))
    if attributes == 0xFFFFFFFF:
        raise ctypes.WinError(ctypes.get_last_error())
    return attributes


def raw_path_mode(path: Path) -> int:
    # Windows lstat can resolve reparse points that are not name surrogates.
    if _windows_file_attributes(path) & stat.FILE_ATTRIBUTE_REPARSE_POINT:
        raise ValueError(f"artifact links are forbidden (Windows reparse point): {path}")
    mode = path.lstat().st_mode
    if stat.S_ISLNK(mode):
        raise ValueError(f"artifact links are forbidden: {path}")
    return mode


def raw_directory_members(root: Path) -> list[tuple[Path, str, bool]]:
    paths = []
    pending = [root]
    while pending:
        directory = pending.pop()
        if not stat.S_ISDIR(raw_path_mode(directory)):
            raise ValueError(f"artifact directory changed: {directory}")
        children = []
        for path in directory.iterdir():
            mode = raw_path_mode(path)
            if stat.S_ISDIR(mode):
                children.append(path)
            paths.append((path, stat.S_ISREG(mode)))
        pending.extend(reversed(children))
    names = _raw_member_paths([(path.relative_to(root).as_posix(), False) for path, _ in paths])
    return [(path, name, regular) for (path, regular), name in zip(paths, names, strict=True)]


def _raw_directory_entries(root: Path, selected: set[str] | None) -> Iterator[tuple[str, bytes]]:
    for path, name, regular in raw_directory_members(root):
        if regular and _raw_member_selected(name, selected):
            yield name, path.read_bytes()


def raw_zip_members(archive: zipfile.ZipFile) -> list[tuple[zipfile.ZipInfo, str]]:
    members = archive.infolist()
    names = _raw_member_paths([(member.orig_filename, stat.S_ISLNK(member.external_attr >> 16)) for member in members])
    return list(zip(members, names, strict=True))


def _raw_zip_entries(path: Path, selected: set[str] | None) -> Iterator[tuple[str, bytes]]:
    with zipfile.ZipFile(path) as archive:
        for member, name in raw_zip_members(archive):
            if not member.is_dir() and _raw_member_selected(name, selected):
                yield name, archive.read(member)


def raw_tar_members(archive: tarfile.TarFile) -> list[tuple[tarfile.TarInfo, str]]:
    members = archive.getmembers()
    names = _raw_member_paths([(member.name, member.issym() or member.islnk()) for member in members])
    for member, name in zip(members, names, strict=True):
        if not member.isfile() and not member.isdir():
            raise ValueError(f"unsupported artifact member: {name}")
    return list(zip(members, names, strict=True))


def _raw_tar_entries(path: Path, selected: set[str] | None) -> Iterator[tuple[str, bytes]]:
    with tarfile.open(path) as archive:
        for member, name in raw_tar_members(archive):
            if member.isfile() and _raw_member_selected(name, selected):
                stream = archive.extractfile(member)
                if stream is None:
                    raise ValueError(f"unreadable artifact member: {name}")
                with stream:
                    yield name, stream.read()


def _raw_artifact_files(artifact: Path, selected: set[str] | None = None) -> dict[str, tuple[str, bytes]]:
    if stat.S_ISDIR(raw_path_mode(artifact)):
        entries = _raw_directory_entries(artifact, selected)
    elif artifact.name.lower().endswith((".tar.gz", ".tgz")):
        entries = _raw_tar_entries(artifact, selected)
    else:
        entries = _raw_zip_entries(artifact, selected)
    return {name.lower(): (name, data) for name, data in entries}


def _php_query(host: Path) -> dict[str, int]:
    code = (
        'echo json_encode(["version_id"=>PHP_VERSION_ID,"zts"=>PHP_ZTS,"debug"=>PHP_DEBUG,"int_size"=>PHP_INT_SIZE]);'
    )
    try:
        result = subprocess.run(
            [str(host), "-n", "-r", code],
            capture_output=True,
            text=True,
            check=False,
            timeout=PHP_QUERY_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError("PHP host command failed") from error
    if result.returncode != 0:
        raise ValueError("PHP host command failed")
    try:
        info = json.loads(result.stdout)
    except (ValueError, TypeError) as error:
        raise ValueError("PHP host JSON is invalid") from error
    fields = ("version_id", "zts", "debug", "int_size")
    if not isinstance(info, dict) or any(type(info.get(field)) is not int for field in fields):
        raise ValueError("PHP host JSON fields must be integers")
    return info


def _php_host_abi(host: Path, version: str | None, machine: int) -> str:
    if version is None or re.fullmatch(r"8\.\d+", version) is None:
        raise ValueError("PHP host version must specify the expected PHP 8 minor")
    image = PeImage(host.read_bytes())
    if image.machine != machine:
        raise ValueError("PHP host PE architecture mismatch")
    if "php8.dll" not in image.imports():
        raise ValueError("PHP host must import php8.dll")
    libraries = [path for path in host.parent.iterdir() if path.is_file() and path.name.lower() == "php8.dll"]
    if len(libraries) != 1:
        raise ValueError("PHP host runtime DLL missing or ambiguous")
    if PeImage(libraries[0].read_bytes()).machine != machine:
        raise ValueError("PHP host PE architecture mismatch")
    info = _php_query(host)
    major, minor = map(int, version.split("."))
    if info["version_id"] // 100 != major * 100 + minor:
        raise ValueError("PHP host version mismatch")
    if info["zts"] != 0:
        raise ValueError("PHP host thread safety must be NTS")
    if info["debug"] != 0:
        raise ValueError("PHP host debug runtime is not supported")
    if info["int_size"] != {0x8664: 8, 0xAA64: 8, 0x14C: 4}.get(machine):
        raise ValueError("PHP host pointer width mismatch")
    return "php8.dll"


def audit_artifact(
    artifact: Path,
    native_glob: str,
    required: str = "onnxruntime.dll",
    forbidden: str = "directml.dll",
    *,
    artifact_kind: str = "wheel",
    php_host: Path | None = None,
    php_version: str | None = None,
    require_crt_notices: bool = False,
) -> dict[str, int]:
    if artifact_kind == "wheel":
        if php_host is not None or php_version is not None:
            raise ValueError("PHP host is only supported for raw artifacts")
        if require_crt_notices:
            raise ValueError("raw CRT notices are only supported for raw artifacts")
        return audit(artifact_files(artifact), native_glob, required, forbidden)
    if artifact_kind != "raw":
        raise ValueError("unknown artifact kind")
    files = _raw_artifact_files(artifact)
    allowed = raw_external_policy(files, roots(files, native_glob), php_host, php_version)
    result = _audit_graph(files, native_glob, required, forbidden, allowed)
    if require_crt_notices:
        audit_raw_crt_notices(artifact, files)
    return result


def raw_external_policy(
    files: dict[str, tuple[str, bytes]],
    native: list[str],
    php_host: Path | None,
    php_version: str | None,
) -> Callable[[str, str], bool]:
    host_abi = None
    if php_host is not None:
        if any(PurePosixPath(key).name != "xberg.dll" for key in native):
            raise ValueError("PHP host requires xberg.dll native roots")
        machines = {PeImage(files[key][1]).machine for key in native}
        if len(machines) != 1:
            raise ValueError("PHP host PE architecture mismatch")
        host_abi = _php_host_abi(php_host, php_version, machines.pop())
    elif php_version is not None:
        raise ValueError("PHP host executable is required")

    def allowed(key: str, name: str) -> bool:
        return (external(name) and re.fullmatch(r"python3(?:\d+)?\.dll", name) is None) or (
            key in native and name == host_abi
        )

    return allowed


def installed_redist_directory() -> Path:
    installed = Path(os.environ.get("PROGRAMFILES(X86)", "C:/Program Files (x86)"))
    tool = installed / "Microsoft Visual Studio/Installer/vswhere.exe"
    try:
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
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError(
            "Visual Studio 2022 discovery failed; a supported release CRT installation is required"
        ) from error
    location = result.stdout.strip()
    if not location:
        raise ValueError("Visual Studio 2022 release CRT installation not found by vswhere")
    installation = Path(location)
    if (
        installation.name not in VS2022_EDITIONS
        or installation.parent.name != "2022"
        or installation.parent.parent.name.casefold() != "microsoft visual studio"
    ):
        raise ValueError("vswhere must identify a supported Visual Studio 2022 installation")
    return installation / "VC/Redist/MSVC"


def crt_directory(machine: int, explicit: str | None) -> Path:
    architecture = {0x8664: "x64", 0xAA64: "arm64", 0x14C: "x86"}.get(machine)
    if architecture is None:
        raise ValueError(f"unsupported PE architecture {machine}")
    source = explicit or os.environ.get("VCToolsRedistDir")  # noqa: SIM112 - Visual Studio documents this exact spelling.
    if not source:
        source = installed_redist_directory()
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


def plan_crt(
    files: dict[str, tuple[str, bytes]],
    source: str | None,
    *,
    verify_shipped_crt: bool = False,
) -> tuple[dict[str, bytes], list[dict[str, str]]]:
    queue = list(files)
    added = {}
    provenance = {}
    directories = {}

    def require(key: str, dependency: str, machine: int) -> None:
        if machine not in directories:
            directories[machine] = crt_directory(machine, source)
        path, data = runtime_file(directories[machine], dependency, machine)
        destination = str(PurePosixPath(key).parent / dependency)
        name = files[destination][0] if destination in files else str(PurePosixPath(files[key][0]).parent / path.name)
        provenance[destination] = {
            "file": name,
            "source": "/".join(path.parts[-7:]),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        if destination in files:
            if files[destination][1] != data:
                raise ValueError(f"CRT byte collision: {dependency}")
            return
        files[destination] = (name, data)
        added[destination] = data
        queue.append(destination)

    while queue:
        key = queue.pop()
        image = PeImage(files[key][1])
        for dependency in image.imports():
            if dependency.startswith(MSVC_PREFIXES):
                require(key, dependency, image.machine)
    if verify_shipped_crt:
        for key, (_, data) in files.items():
            name = PurePosixPath(key).name
            if key not in provenance and name.endswith(".dll") and name.startswith(MSVC_PREFIXES):
                require(key, name, PeImage(data).machine)
    return added, [provenance[key] for key in sorted(provenance)]


def stage(root: Path, native_glob: str, source: str | None) -> dict[str, list[str]]:
    files = artifact_files(root)
    roots(files, native_glob)
    added, provenance = plan_crt(files, source)
    audit(files, native_glob)
    return write_staged(root, files, added, provenance)


def runtime_notice_bytes(provenance: list[dict[str, str]]) -> bytes:
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
    return (json.dumps(payload, indent=2) + "\n").encode()


def audit_raw_crt_notices(artifact: Path, files: dict[str, tuple[str, bytes]]) -> None:
    expected = {}
    for key, (name, data) in files.items():
        path = PurePosixPath(key)
        if path.name.startswith(MSVC_PREFIXES) and path.suffix == ".dll":
            notice = str(path.parent / RAW_CRT_NOTICE.lower())
            expected.setdefault(notice, {})[PurePosixPath(name).name] = hashlib.sha256(data).hexdigest()
    if not expected:
        return
    notices = _raw_artifact_files(artifact, set(expected))
    for key, digests in expected.items():
        if key not in notices:
            raise ValueError(f"missing Microsoft runtime notice: {key}")
        data = notices[key][1]
        payload = json.loads(data)
        rows = payload.get("files") if isinstance(payload, dict) else None
        if not isinstance(rows, list) or any(
            not isinstance(row, dict)
            or set(row) != {"file", "source", "sha256"}
            or any(not isinstance(value, str) for value in row.values())
            for row in rows
        ):
            raise ValueError(f"invalid Microsoft runtime notice: {key}")
        recorded = {row["file"]: row["sha256"] for row in rows}
        if len(recorded) != len(rows) or recorded != digests or data != runtime_notice_bytes(rows):
            raise ValueError(f"Microsoft runtime notice does not match shipped DLLs: {key}")


def notice_plan(root: Path, provenance: list[dict[str, str]]) -> tuple[Path, bytes] | None:
    if not provenance:
        return None
    dist_info = list(root.glob("*.dist-info"))
    if len(dist_info) != 1:
        raise ValueError("expected one wheel dist-info for Microsoft notices")
    notice = dist_info[0] / "licenses/MICROSOFT_VC_RUNTIME.json"
    data = runtime_notice_bytes(provenance)
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
    parser.add_argument("--artifact-kind", choices=["wheel", "raw"], default="wheel")
    parser.add_argument("--php-host", type=Path)
    parser.add_argument("--php-version")
    parser.add_argument("--require-crt-notices", action="store_true")
    parser.add_argument("--required", default="onnxruntime.dll")
    parser.add_argument("--forbidden", default="DirectML.dll")
    args = parser.parse_args()
    try:
        if args.mode == "stage":
            print(json.dumps(stage(args.artifact, args.native_glob, args.crt_dir)))
        else:
            result = audit_artifact(
                args.artifact,
                args.native_glob,
                args.required,
                args.forbidden,
                artifact_kind=args.artifact_kind,
                php_host=args.php_host,
                php_version=args.php_version,
                require_crt_notices=args.require_crt_notices,
            )
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
        tarfile.TarError,
    ) as error:
        parser.exit(1, f"Windows wheel closure: {error}\n")


if __name__ == "__main__":
    main()
