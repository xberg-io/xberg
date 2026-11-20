"""Exercise notice injection against small synthetic distribution archives."""

from __future__ import annotations

import base64
import gzip
import hashlib
import io
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("package-notices.py").resolve()
ROOT = SCRIPT.parents[2]
NOTICES = ("LICENSE", "THIRD_PARTY_LICENSES.md")


def notice_bytes(name: str) -> bytes:
    return (ROOT / name).read_bytes()


def run_script(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )


def record(archive_path: Path) -> dict[str, list[str]]:
    with zipfile.ZipFile(archive_path) as archive:
        payload = archive.read("demo-1.0.0.dist-info/RECORD")
    rows = [line.split(",") for line in payload.decode("utf-8").splitlines() if line]
    return {row[0]: row[1:] for row in rows}


def build_wheel(directory: Path) -> Path:
    path = directory / "demo-1.0.0-py3-none-any.whl"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("demo/__init__.py", "")
        archive.writestr("demo-1.0.0.dist-info/METADATA", "Metadata-Version: 2.4\nName: demo\n")
        archive.writestr("demo-1.0.0.dist-info/WHEEL", "Wheel-Version: 1.0\n")
        archive.writestr(
            "demo-1.0.0.dist-info/RECORD",
            "demo/__init__.py,sha256=abc,0\n"
            "demo-1.0.0.dist-info/METADATA,sha256=def,31\n"
            "demo-1.0.0.dist-info/WHEEL,sha256=ghi,16\n"
            "demo-1.0.0.dist-info/RECORD,,\n",
        )
    return path


def build_jar(directory: Path) -> Path:
    path = directory / "demo-1.0.0.jar"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n")
        archive.writestr("demo/Demo.class", b"\xca\xfe\xba\xbe")
    return path


def build_tarball(directory: Path, *, root: str | None) -> Path:
    path = directory / "demo-1.0.0.tar.gz"
    payload = {"demo.txt": b"payload"}
    with tarfile.open(path, "w:gz") as archive:
        for name, content in payload.items():
            member = tarfile.TarInfo(f"{root}/{name}" if root else name)
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return path


def build_gem(directory: Path) -> Path:
    path = directory / "demo-1.0.0.gem"
    data_buffer = io.BytesIO()
    with (
        gzip.GzipFile(fileobj=data_buffer, mode="wb", mtime=0) as compressed,
        tarfile.open(fileobj=compressed, mode="w") as archive,
    ):
        content = b"puts :demo"
        member = tarfile.TarInfo("lib/demo.rb")
        member.size = len(content)
        archive.addfile(member, io.BytesIO(content))
    data = data_buffer.getvalue()
    metadata = gzip.compress(b"--- !ruby/object:Gem::Specification\n", mtime=0)
    checksums = gzip.compress(b"---\nSHA256: {}\nSHA512: {}\n", mtime=0)
    with tarfile.open(path, "w") as archive:
        for name, content in (
            ("metadata.gz", metadata),
            ("data.tar.gz", data),
            ("checksums.yaml.gz", checksums),
        ):
            member = tarfile.TarInfo(name)
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return path


class PackageNoticesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def test_should_inject_wheel_notices_into_dist_info_and_record(self) -> None:
        wheel = build_wheel(self.root)
        result = run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 0, result.stderr
        with zipfile.ZipFile(wheel) as archive:
            for name in NOTICES:
                packaged = f"demo-1.0.0.dist-info/licenses/{name}"
                assert archive.namelist().count(packaged) == 1
                assert archive.read(packaged) == notice_bytes(name)
        rows = record(wheel)
        for name in NOTICES:
            digest = base64.urlsafe_b64encode(hashlib.sha256(notice_bytes(name)).digest()).decode().rstrip("=")
            assert rows[f"demo-1.0.0.dist-info/licenses/{name}"] == [f"sha256={digest}", str(len(notice_bytes(name)))]
        assert rows["demo-1.0.0.dist-info/RECORD"] == ["", ""]

    def test_should_make_wheel_injection_idempotent(self) -> None:
        wheel = build_wheel(self.root)
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        first = wheel.read_bytes()
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()
        assert len(names) == len(set(names))
        assert names.count("demo-1.0.0.dist-info/licenses/LICENSE") == 1
        assert len(record(wheel)) == 6
        assert first != b""

    def test_should_inject_jar_notices_under_meta_inf(self) -> None:
        jar = build_jar(self.root)
        result = run_script("inject", "--format", "jar", "--expected-count", "1", str(jar))
        assert result.returncode == 0, result.stderr
        with zipfile.ZipFile(jar) as archive:
            for name in NOTICES:
                assert archive.read(f"META-INF/{name}") == notice_bytes(name)
        assert run_script("inject", "--format", "jar", "--expected-count", "1", str(jar)).returncode == 0
        with zipfile.ZipFile(jar) as archive:
            assert len(archive.namelist()) == len(set(archive.namelist()))

    def test_should_inject_sdist_and_zig_notices_at_the_root(self) -> None:
        for archive_format, root in (("sdist", "demo-1.0.0"), ("zig", None)):
            with self.subTest(archive_format=archive_format):
                tarball = build_tarball(self.root, root=root)
                result = run_script("inject", "--format", archive_format, "--expected-count", "1", str(tarball))
                assert result.returncode == 0, result.stderr
                with tarfile.open(tarball) as archive:
                    names = {Path(member.name).name for member in archive.getmembers()}
                    assert set(NOTICES) <= names
                    for name in NOTICES:
                        member = next(item for item in archive.getmembers() if Path(item.name).name == name)
                        assert archive.extractfile(member).read() == notice_bytes(name)
                tarball.unlink()

    def test_should_refresh_gem_data_and_checksums(self) -> None:
        gem = build_gem(self.root)
        result = run_script("inject", "--format", "gem", "--expected-count", "1", str(gem))
        assert result.returncode == 0, result.stderr
        with tarfile.open(gem, "r:") as archive:
            payloads = {}
            for member in archive.getmembers():
                payloads[member.name] = archive.extractfile(member).read()
        with tarfile.open(fileobj=io.BytesIO(payloads["data.tar.gz"]), mode="r:*") as data:
            contents = {
                Path(member.name).name: data.extractfile(member).read()
                for member in data.getmembers()
                if member.isfile()
            }
        for name in NOTICES:
            assert contents[name] == notice_bytes(name)
        assert contents["demo.rb"] == b"puts :demo"
        checksums = gzip.decompress(payloads["checksums.yaml.gz"]).decode("utf-8")
        for algorithm in ("sha256", "sha512"):
            expected = hashlib.new(algorithm, payloads["data.tar.gz"]).hexdigest()
            assert f"  data.tar.gz: {expected}" in checksums
            expected = hashlib.new(algorithm, payloads["metadata.gz"]).hexdigest()
            assert f"  metadata.gz: {expected}" in checksums

    def test_should_fail_on_missing_input_and_wrong_count(self) -> None:
        missing = run_script("inject", "--format", "wheel", "--expected-count", "1", str(self.root / "absent.whl"))
        assert missing.returncode == 1
        assert "archive not found" in missing.stderr
        first = build_wheel(self.root)
        second = self.root / "other-1.0.0-py3-none-any.whl"
        second.write_bytes(first.read_bytes())
        counted = run_script("inject", "--format", "wheel", "--expected-count", "1", str(first), str(second))
        assert counted.returncode == 1
        assert "expected 1 wheel archives, found 2" in counted.stderr

    def test_should_fail_check_when_notice_bytes_differ(self) -> None:
        wheel = build_wheel(self.root)
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        staged = self.root / "staged"
        staged.mkdir()
        assert run_script("stage", str(staged)).returncode == 0
        (staged / "LICENSE").write_bytes(b"tampered")
        with zipfile.ZipFile(wheel) as archive:
            entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
        entries["demo-1.0.0.dist-info/licenses/LICENSE"] = b"tampered"
        with zipfile.ZipFile(wheel, "w") as archive:
            for name, content in entries.items():
                archive.writestr(name, content)
        result = run_script("check", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 1
        assert "differs from the canonical file" in result.stderr

    def test_should_add_notices_to_staged_zig_paths(self) -> None:
        package = self.root / "zig"
        package.mkdir()
        (package / "build.zig.zon").write_text(
            '.{\n    .name = .demo,\n    .paths = .{\n        "build.zig",\n        "src",\n    },\n}\n'
        )
        result = run_script("stage", str(package))
        assert result.returncode == 0, result.stderr
        manifest = (package / "build.zig.zon").read_text()
        assert manifest.count('"LICENSE"') == 1
        assert manifest.count('"THIRD_PARTY_LICENSES.md"') == 1
        assert run_script("stage", str(package)).returncode == 0
        assert (package / "build.zig.zon").read_text().count('"LICENSE"') == 1


if __name__ == "__main__":
    unittest.main()
