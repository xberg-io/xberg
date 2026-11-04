"""Stage the Windows PE closure in a raw tree or atomically replace a verified archive."""

from __future__ import annotations

import argparse
import io
import json
import shutil
import stat
import struct
import subprocess
import tarfile
import tempfile
import zipfile
from collections import defaultdict
from copy import copy
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

import windows_wheel_closure as closure

if TYPE_CHECKING:
    from collections.abc import Callable

REQUIRED = "onnxruntime.dll"
FORBIDDEN = "directml.dll"


def ort_payload(source: Path) -> dict[str, tuple[str, bytes]]:
    payload = {}
    for path in sorted(source.iterdir()):
        key = path.name.lower()
        if not key.endswith(".dll") or key.startswith(closure.MSVC_PREFIXES):
            continue
        if path.is_symlink() or not path.is_file() or key in payload:
            raise ValueError(f"invalid or ambiguous ONNX Runtime source file: {path}")
        payload[key] = (path.name, path.read_bytes())
    if REQUIRED not in payload:
        raise ValueError(f"ONNX Runtime source is missing {REQUIRED}")
    return payload


def plan_raw_stage(
    files: dict[str, tuple[str, bytes]],
    native: list[str],
    dll_source: Path,
    crt_source: str | None,
) -> dict[str, bytes]:
    additions = {}
    runtime = ort_payload(dll_source)
    directories = {PurePosixPath(files[key][0]).parent for key in native}
    for directory in sorted(directories):
        for filename, data in runtime.values():
            relative = str(directory / filename)
            key = relative.lower()
            if key in files:
                if files[key][1] != data:
                    raise ValueError(f"ONNX Runtime byte collision: {relative}")
                continue
            files[key] = (relative, data)
            additions[relative] = data
    added_crt, provenance = closure.plan_crt(files, crt_source, verify_shipped_crt=True)
    additions.update((files[key][0], data) for key, data in added_crt.items())
    notices = defaultdict(list)
    for entry in provenance:
        path = PurePosixPath(entry["file"])
        notices[path.parent].append({**entry, "file": path.name})
    for directory, rows in notices.items():
        additions[str(directory / closure.RAW_CRT_NOTICE)] = closure.runtime_notice_bytes(rows)
    return additions


def pending_additions(additions: dict[str, bytes]) -> dict[str, tuple[str, bytes]]:
    closure._raw_member_paths([(name, False) for name in additions])
    return {name.casefold(): (name, data) for name, data in additions.items()}


def check_existing(name: str, actual: bytes | None, pending: dict[str, tuple[str, bytes]]) -> None:
    addition = pending.pop(name.casefold(), None)
    if addition is not None and actual != addition[1]:
        raise ValueError(f"staged artifact member collision: {name}")


def write_zip(source: Path, destination: Path, pending: dict[str, tuple[str, bytes]]) -> None:
    with (
        zipfile.ZipFile(source) as old,
        zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as new,
    ):
        new.comment = old.comment
        for member, name in closure.raw_zip_members(old):
            if name.casefold() in pending:
                check_existing(name, None if member.is_dir() else old.read(member), pending)
            with old.open(member) as data, new.open(copy(member), "w") as target:
                shutil.copyfileobj(data, target)
        for name, data in pending.values():
            new.writestr(name, data)


def write_tar(source: Path, destination: Path, pending: dict[str, tuple[str, bytes]]) -> None:
    with tarfile.open(source) as old, tarfile.open(destination, "w:gz") as new:
        for member, name in closure.raw_tar_members(old):
            if member.isdir():
                check_existing(name, None, pending)
                new.addfile(member)
                continue
            stream = old.extractfile(member)
            if stream is None:
                raise ValueError(f"unreadable artifact member: {name}")
            with stream:
                if name.casefold() in pending:
                    check_existing(name, stream.read(), pending)
                    stream.seek(0)
                new.addfile(member, stream)
        for name, data in pending.values():
            member = tarfile.TarInfo(name)
            member.mode = 0o644
            member.size = len(data)
            new.addfile(member, io.BytesIO(data))


def write_directory(
    root: Path,
    pending: dict[str, tuple[str, bytes]],
    verify: Callable[[Path], None],
) -> None:
    for path, name, regular in closure.raw_directory_members(root):
        if name.casefold() in pending:
            check_existing(name, path.read_bytes() if regular else None, pending)
    written = []
    try:
        for name, data in pending.values():
            path = root / name
            with path.open("xb") as stream:
                written.append(path)
                stream.write(data)
        verify(root)
    except (OSError, ValueError, UnicodeError, struct.error):
        for path in reversed(written):
            path.unlink()
        raise


def stage_raw(
    artifact: Path,
    native_glob: str,
    dll_source: Path,
    crt_source: str | None,
    *,
    php_host: Path | None,
    php_version: str | None,
) -> None:
    is_directory = stat.S_ISDIR(closure.raw_path_mode(artifact))
    name = artifact.name.lower()
    if not is_directory and not name.endswith((".zip", ".tgz", ".tar.gz")):
        raise ValueError(f"unsupported raw archive type: {artifact}")
    files = closure._raw_artifact_files(artifact)
    native = closure.roots(files, native_glob)
    allowed = closure.raw_external_policy(files, native, php_host, php_version)
    additions = plan_raw_stage(files, native, dll_source, crt_source)
    closure._audit_graph(files, native_glob, REQUIRED, FORBIDDEN, allowed)

    def verify(path: Path) -> None:
        final_files = closure._raw_artifact_files(path)
        closure._audit_graph(final_files, native_glob, REQUIRED, FORBIDDEN, allowed)
        closure.audit_raw_crt_notices(path, final_files)

    pending = pending_additions(additions)
    if is_directory:
        write_directory(artifact, pending, verify)
        return
    with tempfile.TemporaryDirectory(prefix=".windows-closure-", dir=artifact.parent) as scratch:
        staged = Path(scratch) / artifact.name
        write_archive = write_zip if name.endswith(".zip") else write_tar
        write_archive(artifact, staged, pending)
        verify(staged)
        staged.replace(artifact)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--native-glob", required=True)
    parser.add_argument("--dll-source-dir", type=Path, required=True)
    parser.add_argument("--crt-dir")
    parser.add_argument("--php-host", type=Path)
    parser.add_argument("--php-version")
    args = parser.parse_args()
    try:
        stage_raw(
            args.artifact,
            args.native_glob,
            args.dll_source_dir,
            args.crt_dir,
            php_host=args.php_host,
            php_version=args.php_version,
        )
    except (
        OSError,
        ValueError,
        UnicodeError,
        struct.error,
        subprocess.SubprocessError,
        tarfile.TarError,
        zipfile.BadZipFile,
    ) as error:
        parser.exit(1, f"Windows raw closure: {error}\n")
    print(json.dumps({"staged": str(args.artifact)}))


if __name__ == "__main__":
    main()
