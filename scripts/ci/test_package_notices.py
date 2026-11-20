"""Exercise notice injection against small synthetic distribution archives."""

from __future__ import annotations

import base64
import csv
import gzip
import hashlib
import io
import shutil
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


def record_rows(archive_path: Path) -> list[list[str]]:
    with zipfile.ZipFile(archive_path) as archive:
        payload = archive.read("demo-1.0.0.dist-info/RECORD")
    return [row for row in csv.reader(io.StringIO(payload.decode("utf-8"))) if row]


def digest(content: bytes) -> str:
    return base64.urlsafe_b64encode(hashlib.sha256(content).digest()).decode().rstrip("=")


def build_wheel(directory: Path) -> Path:
    path = directory / "demo-1.0.0-py3-none-any.whl"
    files = {
        "demo/__init__.py": b"VALUE = 1\n",
        "demo-1.0.0.dist-info/METADATA": b"Metadata-Version: 2.4\nName: demo\nVersion: 1.0.0\n\nBody.\n",
        "demo-1.0.0.dist-info/WHEEL": b"Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\n",
    }
    rows = [[name, f"sha256={digest(content)}", str(len(content))] for name, content in files.items()]
    rows.append(["demo-1.0.0.dist-info/RECORD", "", ""])
    buffer = io.StringIO()
    csv.writer(buffer, lineterminator="\n").writerows(rows)
    files["demo-1.0.0.dist-info/RECORD"] = buffer.getvalue().encode()
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in files.items():
            archive.writestr(name, content)
    return path


def build_jar(directory: Path) -> Path:
    path = directory / "demo-1.0.0.jar"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n")
        archive.writestr("demo/Demo.class", b"\xca\xfe\xba\xbe")
    return path


def build_nupkg(directory: Path) -> Path:
    path = directory / "Demo.1.0.0.nupkg"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("Demo.nuspec", "<package />")
        archive.writestr("lib/net8.0/Demo.dll", b"assembly")
    return path


def build_tarball(directory: Path, *, roots: tuple[str, ...] = ("demo-1.0.0",)) -> Path:
    path = directory / "demo-1.0.0.tar.gz"
    with tarfile.open(path, "w:gz") as archive:
        for root in roots:
            content = f"payload for {root}".encode()
            member = tarfile.TarInfo(f"{root}/demo.txt" if root else "demo.txt")
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return path


def build_sdist(directory: Path) -> Path:
    path = build_tarball(directory)
    pkg_info = b"Metadata-Version: 2.4\nName: demo\nVersion: 1.0.0\n\nBody.\n"
    pyproject = (
        b'[build-system]\nrequires = ["hatchling"]\nbuild-backend = "hatchling.build"\n\n'
        b'[project]\nname = "demo"\nversion = "1.0.0"\nlicense = "MIT"\nlicense-files = ["LICENSE"]\n'
    )
    with tarfile.open(path, "w:gz") as archive:
        for name, content in (
            ("demo-1.0.0/PKG-INFO", pkg_info),
            ("demo-1.0.0/pyproject.toml", pyproject),
            ("demo-1.0.0/demo.txt", b"payload"),
        ):
            member = tarfile.TarInfo(name)
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return path


def build_gem(directory: Path) -> Path:
    path = directory / "demo-1.0.0.gem"
    data_buffer = io.BytesIO()
    with (
        gzip.GzipFile(fileobj=data_buffer, mode="wb", mtime=0) as compressed,
        tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as archive,
    ):
        content = b"puts :demo"
        member = tarfile.TarInfo("lib/demo.rb")
        member.size = len(content)
        archive.addfile(member, io.BytesIO(content))
    data = data_buffer.getvalue()
    metadata = gzip.compress(b"--- !ruby/object:Gem::Specification\n", mtime=0)
    checksums = gzip.compress(b"---\nSHA256: {}\nSHA512: {}\n", mtime=0)
    with tarfile.open(path, "w", format=tarfile.USTAR_FORMAT) as archive:
        for name, content in (
            ("metadata.gz", metadata),
            ("data.tar.gz", data),
            ("checksums.yaml.gz", checksums),
        ):
            member = tarfile.TarInfo(name)
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return path


def rewrite_zip(path: Path, updates: dict[str, bytes], *, extra: dict[str, bytes] | None = None) -> None:
    with zipfile.ZipFile(path) as archive:
        entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
    entries.update(updates)
    entries.update(extra or {})
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in entries.items():
            archive.writestr(name, content)


class PackageNoticesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def test_should_inject_wheel_notices_and_declare_them_in_metadata(self) -> None:
        wheel = build_wheel(self.root)
        result = run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 0, result.stderr
        with zipfile.ZipFile(wheel) as archive:
            for name in NOTICES:
                packaged = f"demo-1.0.0.dist-info/licenses/{name}"
                assert archive.namelist().count(packaged) == 1
                assert archive.read(packaged) == notice_bytes(name)
            metadata = archive.read("demo-1.0.0.dist-info/METADATA").decode()
        assert "License-File: LICENSE" in metadata
        assert "License-File: THIRD_PARTY_LICENSES.md" in metadata
        rows = {row[0]: row[1:] for row in record_rows(wheel)}
        for name in NOTICES:
            packaged = f"demo-1.0.0.dist-info/licenses/{name}"
            assert rows[packaged] == [f"sha256={digest(notice_bytes(name))}", str(len(notice_bytes(name)))]
        assert rows["demo-1.0.0.dist-info/RECORD"] == ["", ""]
        assert rows["demo-1.0.0.dist-info/METADATA"][0].startswith("sha256=")

    def test_should_make_wheel_injection_idempotent(self) -> None:
        wheel = build_wheel(self.root)
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()
            metadata = archive.read("demo-1.0.0.dist-info/METADATA").decode()
        assert len(names) == len(set(names))
        assert names.count("demo-1.0.0.dist-info/licenses/THIRD_PARTY_LICENSES.md") == 1
        assert metadata.count("License-File: THIRD_PARTY_LICENSES.md") == 1
        rows = record_rows(wheel)
        assert len(rows) == len({row[0] for row in rows}) == 6

    def test_should_reject_a_stale_record(self) -> None:
        wheel = build_wheel(self.root)
        assert run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel)).returncode == 0
        rows = record_rows(wheel)
        for row in rows:
            if row[0] == "demo-1.0.0.dist-info/METADATA":
                row[1] = "sha256=" + "A" * 43
        buffer = io.StringIO()
        csv.writer(buffer, lineterminator="\n").writerows(rows)
        rewrite_zip(wheel, {"demo-1.0.0.dist-info/RECORD": buffer.getvalue().encode()})
        result = run_script("check", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 1
        assert "RECORD hash mismatch" in result.stderr

    def test_should_reject_a_wheel_that_omits_a_record_row(self) -> None:
        wheel = build_wheel(self.root)
        rows = [row for row in record_rows(wheel) if row[0] != "demo-1.0.0.dist-info/WHEEL"]
        buffer = io.StringIO()
        csv.writer(buffer, lineterminator="\n").writerows(rows)
        rewrite_zip(wheel, {"demo-1.0.0.dist-info/RECORD": buffer.getvalue().encode()})
        result = run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 1
        assert "RECORD does not cover the archive exactly" in result.stderr

    def test_should_refuse_signed_wheels_jars_and_gems(self) -> None:
        wheel = build_wheel(self.root)
        rewrite_zip(wheel, {}, extra={"demo-1.0.0.dist-info/RECORD.jws": b"signature"})
        result = run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 1
        assert "is signed" in result.stderr

        jar = build_jar(self.root)
        rewrite_zip(jar, {}, extra={"META-INF/TEST.SF": b"Signature-Version: 1.0\n"})
        result = run_script("inject", "--format", "jar", "--expected-count", "1", str(jar))
        assert result.returncode == 1
        assert "is signed" in result.stderr

        gem = build_gem(self.root)
        with tarfile.open(gem, "r:") as archive:
            members = {member.name: archive.extractfile(member).read() for member in archive.getmembers()}
        members["metadata.gz.sig"] = b"signature"
        with tarfile.open(gem, "w", format=tarfile.USTAR_FORMAT) as archive:
            for name, content in members.items():
                member = tarfile.TarInfo(name)
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
        result = run_script("inject", "--format", "gem", "--expected-count", "1", str(gem))
        assert result.returncode == 1
        assert "is signed" in result.stderr

    def test_should_inject_jar_and_nupkg_notices(self) -> None:
        jar = build_jar(self.root)
        result = run_script("inject", "--format", "jar", "--expected-count", "1", str(jar))
        assert result.returncode == 0, result.stderr
        with zipfile.ZipFile(jar) as archive:
            for name in NOTICES:
                assert archive.read(f"META-INF/{name}") == notice_bytes(name)
        assert run_script("inject", "--format", "jar", "--expected-count", "1", str(jar)).returncode == 0
        with zipfile.ZipFile(jar) as archive:
            assert len(archive.namelist()) == len(set(archive.namelist()))

        nupkg = build_nupkg(self.root)
        result = run_script("inject", "--format", "nuget", "--expected-count", "1", str(nupkg))
        assert result.returncode == 0, result.stderr
        with zipfile.ZipFile(nupkg) as archive:
            for name in NOTICES:
                assert archive.read(name) == notice_bytes(name)
        assert run_script("inject", "--format", "nuget", "--expected-count", "1", str(nupkg)).returncode == 0

    def test_should_inject_sdist_notices_declare_them_and_reject_multiple_roots(self) -> None:
        sdist = build_sdist(self.root)
        result = run_script("inject", "--format", "sdist", "--expected-count", "1", str(sdist))
        assert result.returncode == 0, result.stderr
        with tarfile.open(sdist) as archive:
            contents = {
                Path(member.name).relative_to("demo-1.0.0").as_posix(): archive.extractfile(member).read()
                for member in archive.getmembers()
                if member.isfile()
            }
        for name in NOTICES:
            assert contents[name] == notice_bytes(name)
        assert b"License-File: THIRD_PARTY_LICENSES.md" in contents["PKG-INFO"]
        assert b'"THIRD_PARTY_LICENSES.md"' in contents["pyproject.toml"]

        multi = self.root / "multi"
        multi.mkdir()
        broken = build_tarball(multi, roots=("first", "second"))
        result = run_script("inject", "--format", "sdist", "--expected-count", "1", str(broken))
        assert result.returncode == 1
        assert "must have one root" in result.stderr

    def test_should_inject_zig_notices_at_the_root(self) -> None:
        tarball = build_tarball(self.root, roots=("",))
        result = run_script("inject", "--format", "zig", "--expected-count", "1", str(tarball))
        assert result.returncode == 0, result.stderr
        with tarfile.open(tarball) as archive:
            names = {Path(member.name).name for member in archive.getmembers()}
            assert set(NOTICES) <= names

    def test_should_refresh_gem_data_and_checksums(self) -> None:
        gem = build_gem(self.root)
        result = run_script("inject", "--format", "gem", "--expected-count", "1", str(gem))
        assert result.returncode == 0, result.stderr
        with tarfile.open(gem, "r:") as archive:
            payloads = {member.name: archive.extractfile(member).read() for member in archive.getmembers()}
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

    def test_should_install_the_injected_wheel_with_pip(self) -> None:
        if subprocess.run([sys.executable, "-m", "pip", "--version"], capture_output=True, check=False).returncode:
            self.skipTest("pip is not available")
        wheel = build_wheel(self.root)
        result = run_script("inject", "--format", "wheel", "--expected-count", "1", str(wheel))
        assert result.returncode == 0, result.stderr
        target = self.root / "installed"
        installed = subprocess.run(
            [sys.executable, "-m", "pip", "install", "--quiet", "--no-deps", "--target", str(target), str(wheel)],
            capture_output=True,
            text=True,
            check=False,
        )
        assert installed.returncode == 0, installed.stderr
        licenses = target / "demo-1.0.0.dist-info" / "licenses"
        for name in NOTICES:
            assert (licenses / name).read_bytes() == notice_bytes(name)

    def test_should_install_the_injected_gem_with_gem_install(self) -> None:
        gem_command = shutil.which("gem")
        if gem_command is None:
            self.skipTest("gem is not available")
        source = self.root / "source"
        (source / "lib").mkdir(parents=True)
        (source / "lib" / "demo.rb").write_text('module Demo; VERSION = "1.0.0"; end\n')
        (source / "demo.gemspec").write_text(
            "Gem::Specification.new do |spec|\n"
            '  spec.name = "demo"\n  spec.version = "1.0.0"\n  spec.summary = "demo"\n'
            '  spec.authors = ["test"]\n  spec.license = "MIT"\n  spec.files = ["lib/demo.rb"]\n'
            "end\n"
        )
        built = subprocess.run(
            [gem_command, "build", "demo.gemspec"], cwd=source, capture_output=True, text=True, check=False
        )
        assert built.returncode == 0, built.stderr
        gem = source / "demo-1.0.0.gem"
        result = run_script("inject", "--format", "gem", "--expected-count", "1", str(gem))
        assert result.returncode == 0, result.stderr
        installed = subprocess.run(
            [gem_command, "install", "--local", "--no-document", "--install-dir", str(self.root / "gems"), str(gem)],
            capture_output=True,
            text=True,
            check=False,
        )
        assert installed.returncode == 0, installed.stderr
        for name in NOTICES:
            assert (self.root / "gems" / "gems" / "demo-1.0.0" / name).read_bytes() == notice_bytes(name)

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
