#!/usr/bin/env python3
"""Stage canonical distribution notices and inject/verify final archive bytes."""

import argparse
import base64
import csv
import gzip
import hashlib
import io
import json
import shutil
import sys
import tarfile
import zipfile
from collections.abc import Iterable
from pathlib import Path, PurePosixPath

NOTICE_NAMES = ("LICENSE", "THIRD_PARTY_LICENSES.md")
ARCHIVE_FORMATS = ("wheel", "sdist", "jar", "nuget", "gem", "zig")
ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)


def canonical_notices() -> dict[str, bytes]:
    root = Path(__file__).resolve().parents[2]
    notices = {}
    for name in NOTICE_NAMES:
        content = (root / name).read_bytes()
        if not content:
            raise ValueError(f"empty canonical notice: {name}")
        notices[name] = content
    return notices


def _stage_zig_paths(manifest: Path, names: Iterable[str]) -> None:
    text = manifest.read_text()
    marker = ".paths = .{"
    start = text.find(marker)
    if start < 0:
        raise ValueError(f"no .paths block in {manifest}")
    depth = 0
    end = len(text)
    for index in range(start + len(marker) - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                end = index
                break
    block = text[start:end]
    missing = [name for name in names if f'"{name}"' not in block]
    if not missing:
        return
    insertion = "".join(f'\n        "{name}",' for name in missing)
    manifest.write_text(text[: start + len(marker)] + insertion + text[start + len(marker) :])


def stage(destinations: list[str], notices: dict[str, bytes]) -> None:
    root = Path(__file__).resolve().parents[2]
    for name in destinations:
        destination = Path(name)
        destination.mkdir(parents=True, exist_ok=True)
        for notice in notices:
            shutil.copyfile(root / notice, destination / notice)
        manifest = destination / "package.json"
        if manifest.is_file():
            package = json.loads(manifest.read_text())
            package["files"] = sorted(set(package.get("files") or []) | set(notices))
            manifest.write_text(json.dumps(package, indent=2) + "\n")
        zon = destination / "build.zig.zon"
        if zon.is_file():
            _stage_zig_paths(zon, notices)
    print(f"Staged {len(notices) * len(destinations)} canonical notice files")


def _zip_rewrite(path: Path, changes: dict[str, bytes], remove: set[str]) -> None:
    temporary = path.with_name(f".{path.name}.notices")
    with (
        zipfile.ZipFile(path) as source,
        zipfile.ZipFile(temporary, "w") as destination,
    ):
        for entry in source.infolist():
            if entry.filename in remove:
                continue
            written = zipfile.ZipInfo(entry.filename, date_time=entry.date_time)
            written.compress_type = entry.compress_type
            written.external_attr = entry.external_attr
            written.internal_attr = entry.internal_attr
            written.create_system = entry.create_system
            written.extra = entry.extra
            destination.writestr(written, source.read(entry))
        for name, content in changes.items():
            written = zipfile.ZipInfo(name, date_time=ZIP_EPOCH)
            written.compress_type = zipfile.ZIP_DEFLATED
            written.external_attr = 0o100644 << 16
            destination.writestr(written, content)
    temporary.replace(path)


def _tar_rewrite(path: Path, additions: dict[str, bytes], replacement: dict[str, bytes], root: str) -> None:
    temporary = path.with_name(f".{path.name}.notices")
    targets = {PurePosixPath(root, notice).as_posix() for notice in additions}
    with (
        tarfile.open(path, "r:*") as source,
        temporary.open("wb") as raw,
        gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as compressed,
        tarfile.open(fileobj=compressed, mode="w", format=source.format) as destination,
    ):
        for member in source.getmembers():
            name = PurePosixPath(member.name).as_posix()
            if name in replacement or name in targets:
                continue
            stream = source.extractfile(member) if member.isfile() else None
            destination.addfile(member, stream)
        for name, content in additions.items():
            info = tarfile.TarInfo(PurePosixPath(root, name).as_posix())
            info.size = len(content)
            info.mtime = 0
            info.mode = 0o644
            destination.addfile(info, io.BytesIO(content))
    temporary.replace(path)


def _single_tar_root(names: Iterable[str]) -> str:
    roots = {PurePosixPath(name).parts[0] for name in names if name not in {".", ""} and PurePosixPath(name).parts}
    if len(roots) != 1:
        raise ValueError(f"archive must have one root, found {sorted(roots)}")
    return roots.pop()


def _tar_member(source: bytes | Path, name: str) -> bytes | None:
    """Read one member without materialising the whole (possibly multi-GB) archive."""
    archive = (
        tarfile.open(fileobj=io.BytesIO(source), mode="r:*")
        if isinstance(source, bytes)
        else tarfile.open(source, "r:*")
    )
    with archive:
        matches = [member for member in archive.getmembers() if PurePosixPath(member.name) == PurePosixPath(name)]
        if len(matches) > 1:
            raise ValueError(f"duplicate archive entry: {name}")
        if not matches:
            return None
        stream = archive.extractfile(matches[0])
        if stream is None:
            raise ValueError(f"unreadable archive member: {name}")
        with stream:
            return stream.read()


def _record_rows(payload: bytes) -> list[list[str]]:
    return [row for row in csv.reader(io.StringIO(payload.decode("utf-8"))) if row]


def _record_digest(content: bytes) -> str:
    return base64.urlsafe_b64encode(hashlib.sha256(content).digest()).decode("ascii").rstrip("=")


def _inject_wheel(path: Path, notices: dict[str, bytes]) -> None:
    with zipfile.ZipFile(path) as archive:
        records = [name for name in archive.namelist() if name.count("/") == 1 and name.endswith(".dist-info/RECORD")]
        if len(records) != 1:
            raise ValueError(f"expected one .dist-info/RECORD in {path.name}, found {len(records)}")
        record_path = records[0]
        signed = [name for name in archive.namelist() if name in {f"{record_path}.jws", f"{record_path}.p7s"}]
        if signed:
            raise ValueError(f"{path.name} is signed; rewriting RECORD would invalidate {signed[0]}")
        rows = _record_rows(archive.read(record_path))
    license_root = f"{PurePosixPath(record_path).parent}/licenses"
    additions = {f"{license_root}/{name}": content for name, content in notices.items()}
    retained = [row for row in rows if row[0] not in additions and row[0] != record_path]
    for name, content in additions.items():
        retained.append([name, f"sha256={_record_digest(content)}", str(len(content))])
    retained.append([record_path, "", ""])
    buffer = io.StringIO()
    csv.writer(buffer, lineterminator="\n").writerows(retained)
    _zip_rewrite(path, {**additions, record_path: buffer.getvalue().encode()}, {*additions, record_path})


def _inject_zip_root(path: Path, notices: dict[str, bytes], prefix: str) -> None:
    additions = {f"{prefix}{name}": content for name, content in notices.items()}
    _zip_rewrite(path, additions, set(additions))


def _inject_sdist(path: Path, notices: dict[str, bytes]) -> None:
    with tarfile.open(path, "r:*") as archive:
        root = _single_tar_root(member.name for member in archive.getmembers())
    _tar_rewrite(path, notices, {}, root)


def _gem_checksums(metadata: bytes, data: bytes) -> bytes:
    lines = ["---"]
    for algorithm in ("sha256", "sha512"):
        lines.append(f"{algorithm.upper()}:")
        for name, payload in (("metadata.gz", metadata), ("data.tar.gz", data)):
            lines.append(f"  {name}: {hashlib.new(algorithm, payload).hexdigest()}")
    return gzip.compress(("\n".join(lines) + "\n").encode("utf-8"), mtime=0)


def _inject_gem(path: Path, notices: dict[str, bytes]) -> None:
    with tarfile.open(path, "r:") as archive:
        order = [member.name for member in archive.getmembers()]
        members = {member.name: member for member in archive.getmembers()}
        streams = {name: archive.extractfile(member) for name, member in members.items() if member.isfile()}
        payloads = {}
        for name, stream in streams.items():
            if stream is None:
                raise ValueError(f"unreadable gem member: {name}")
            with stream:
                payloads[name] = stream.read()
    for required in ("metadata.gz", "data.tar.gz", "checksums.yaml.gz"):
        if required not in payloads:
            raise ValueError(f"gem is missing {required}: {path.name}")
    data = payloads["data.tar.gz"]
    data_path = path.with_name(f".{path.name}.data.tar.gz")
    data_path.write_bytes(data)
    try:
        _tar_rewrite(data_path, notices, {}, "")
        rewritten = data_path.read_bytes()
    finally:
        data_path.unlink(missing_ok=True)
    payloads["data.tar.gz"] = rewritten
    payloads["checksums.yaml.gz"] = _gem_checksums(payloads["metadata.gz"], rewritten)
    temporary = path.with_name(f".{path.name}.notices")
    with tarfile.open(path, "r:") as source, tarfile.open(temporary, "w", format=source.format) as destination:
        for name in order:
            member = members[name]
            if member.isfile():
                member.size = len(payloads[name])
                destination.addfile(member, io.BytesIO(payloads[name]))
            else:
                destination.addfile(member)
    temporary.replace(path)


def _inject(path: Path, archive_format: str, notices: dict[str, bytes]) -> None:
    if archive_format == "wheel":
        _inject_wheel(path, notices)
    elif archive_format == "sdist":
        _inject_sdist(path, notices)
    elif archive_format == "jar":
        _inject_zip_root(path, notices, "META-INF/")
    elif archive_format == "nuget":
        _inject_zip_root(path, notices, "")
    elif archive_format == "gem":
        _inject_gem(path, notices)
    else:
        _tar_rewrite(path, notices, {}, "")
    print(f"Injected {len(notices)} canonical notices into {path}")


def _zip_entry(path: Path, name: str) -> bytes | None:
    with zipfile.ZipFile(path) as archive:
        matches = [entry for entry in archive.namelist() if PurePosixPath(entry) == PurePosixPath(name)]
        if len(matches) > 1:
            raise ValueError(f"duplicate archive entry: {name}")
        return archive.read(matches[0]) if matches else None


def _check(path: Path, archive_format: str, notices: dict[str, bytes]) -> None:
    if archive_format == "wheel":
        with zipfile.ZipFile(path) as archive:
            records = [
                name for name in archive.namelist() if name.count("/") == 1 and name.endswith(".dist-info/RECORD")
            ]
        if len(records) != 1:
            raise ValueError(f"expected one .dist-info/RECORD in {path.name}, found {len(records)}")
        prefix = f"{PurePosixPath(records[0]).parent}/licenses"
        reader = lambda name: _zip_entry(path, f"{prefix}/{name}")  # noqa: E731
    elif archive_format == "jar":
        reader = lambda name: _zip_entry(path, f"META-INF/{name}")  # noqa: E731
    elif archive_format == "nuget":
        reader = lambda name: _zip_entry(path, name)  # noqa: E731
    elif archive_format == "gem":
        with tarfile.open(path, "r:") as archive:
            member = archive.getmember("data.tar.gz")
            stream = archive.extractfile(member)
            if stream is None:
                raise ValueError(f"unreadable data.tar.gz in {path.name}")
            with stream:
                data = stream.read()
        reader = lambda name: _tar_member(data, name)  # noqa: E731
    else:
        with tarfile.open(path, "r:*") as archive:
            root = _single_tar_root(member.name for member in archive.getmembers()) if archive_format == "sdist" else ""
        prefix = f"{root}/" if root else ""
        reader = lambda name: _tar_member(path, f"{prefix}{name}")  # noqa: E731
    for name, expected in notices.items():
        content = reader(name)
        if content != expected:
            raise ValueError(f"{path.name}: {name} is missing or differs from the canonical file")
    print(f"Verified {len(notices)} canonical notices in {path}")


def inject(archives: list[str], archive_format: str, expected: int, notices: dict[str, bytes]) -> None:
    if len(archives) != expected:
        raise ValueError(f"expected {expected} {archive_format} archives, found {len(archives)}")
    for name in archives:
        path = Path(name)
        if not path.is_file():
            raise ValueError(f"archive not found: {name}")
        _inject(path, archive_format, notices)
        _check(path, archive_format, notices)


def check(archives: list[str], archive_format: str, expected: int, notices: dict[str, bytes]) -> None:
    if len(archives) != expected:
        raise ValueError(f"expected {expected} {archive_format} archives, found {len(archives)}")
    for name in archives:
        path = Path(name)
        if not path.is_file():
            raise ValueError(f"archive not found: {name}")
        _check(path, archive_format, notices)


def verify(archive_path: str, prefixes: list[str], notices: dict[str, bytes]) -> None:
    with zipfile.ZipFile(archive_path) if zipfile.is_zipfile(archive_path) else tarfile.open(archive_path) as archive:
        if isinstance(archive, zipfile.ZipFile):
            members = [(member.filename, member) for member in archive.infolist() if not member.is_dir()]
        else:
            members = [(member.name, member) for member in archive.getmembers() if member.isfile()]
        for prefix in prefixes:
            for notice, expected in notices.items():
                path = PurePosixPath(prefix) / notice
                matches = [member for name, member in members if PurePosixPath(name) == path]
                if len(matches) != 1:
                    raise ValueError(f"expected one {path}, found {len(matches)}")
                if isinstance(archive, zipfile.ZipFile):
                    content = archive.read(matches[0])
                else:
                    stream = archive.extractfile(matches[0])
                    if stream is None:
                        raise ValueError(f"unreadable archive notice: {path}")
                    with stream:
                        content = stream.read()
                if content != expected:
                    raise ValueError(f"{path} differs from canonical {notice}")
    print(f"Verified {len(notices) * len(prefixes)} canonical notice files")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("stage").add_argument("destinations", nargs="+")
    verification = commands.add_parser("verify")
    verification.add_argument("archive")
    verification.add_argument("prefixes", nargs="+")
    for name in ("inject", "check"):
        command = commands.add_parser(name)
        command.add_argument("--format", dest="archive_format", choices=ARCHIVE_FORMATS, required=True)
        command.add_argument("--expected-count", type=int, required=True)
        command.add_argument("archives", nargs="+")
    args = parser.parse_args()
    try:
        notices = canonical_notices()
        if args.command == "stage":
            stage(args.destinations, notices)
        elif args.command == "verify":
            verify(args.archive, args.prefixes, notices)
        elif args.command == "inject":
            inject(args.archives, args.archive_format, args.expected_count, notices)
        else:
            check(args.archives, args.archive_format, args.expected_count, notices)
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
