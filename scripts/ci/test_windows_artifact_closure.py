"""Qualify raw artifact readers and host boundaries with portable PE fixtures."""

import io
import json
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

import windows_wheel_closure as closure
from test_windows_wheel_closure import expected_error, pe

LAYOUTS = {
    "php": ("xberg.dll", ""),
    "cli": ("xberg.exe", "xberg-cli-x86_64-pc-windows-msvc"),
    "ffi": ("xberg_ffi.dll", "xberg-ffi-windows-x86_64/lib"),
    "nif": ("libxberg_nif-v1.3.10-nif-2.16-x86_64-pc-windows-msvc.dll", ""),
}


class RawArtifactTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.stage = self.root / "stage"
        self.stage.mkdir()
        self.native = self.stage / "xberg.dll"
        self.native.write_bytes(pe(["onnxruntime.dll", "KERNEL32.dll"]))
        (self.stage / "onnxruntime.dll").write_bytes(pe(["vcruntime140.dll"]))
        (self.stage / "vcruntime140.dll").write_bytes(pe(["KERNEL32.dll"]))

    def qualify(self, expected: int = 3) -> None:
        files = [p for p in self.stage.rglob("*") if p.suffix.lower() in (".exe", ".dll")]
        assert len(files) == expected
        for path in files:
            data = path.read_bytes()
            assert data[:2] == b"MZ"
            assert struct.unpack_from("<H", data, 0x84)[0] == 0x8664
            assert len(closure.PeImage(data).imports()) >= 1

    def layout(self, kind: str) -> tuple[str, Path]:
        assert len(LAYOUTS) == 4
        name, directory = LAYOUTS[kind]
        target = self.stage / directory
        target.mkdir(parents=True, exist_ok=True)
        self.native.rename(self.stage / "temporary.dll")
        for path in list(self.stage.glob("*.dll")):
            path.rename(target / (name if path.name == "temporary.dll" else path.name))
        self.native = target / name
        self.qualify()
        return name, target

    def archive(self, kind: str, suffix: str) -> tuple[Path, str]:
        native, _ = self.layout(kind)
        members = {
            str(p.relative_to(self.stage)).replace("\\", "/"): p.read_bytes()
            for p in self.stage.rglob("*")
            if p.is_file()
        }
        assert len(members) == 3
        artifact = self.root / (kind + suffix)
        if suffix == ".zip":
            with zipfile.ZipFile(artifact, "w") as output:
                for name, data in members.items():
                    output.writestr(name, data)
            with zipfile.ZipFile(artifact) as source:
                assert set(source.namelist()) == set(members)
        else:
            with tarfile.open(artifact, "w:gz") as output:
                for name, data in members.items():
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    output.addfile(info, io.BytesIO(data))
            with tarfile.open(artifact) as source:
                assert {m.name for m in source.getmembers()} == set(members)
        return artifact, native

    def audit(self, artifact: Path | None = None, native: str = "xberg.dll", **kwargs: object) -> dict[str, int]:
        return closure.audit_artifact(artifact or self.stage, native, artifact_kind="raw", **kwargs)

    def require_complete(self, artifact: Path | None = None, native: str = "xberg.dll") -> None:
        assert self.audit(artifact, native) == {"files": 3, "edges": 4, "roots": 1}

    def test_php_zip_has_complete_valid_raw_graph(self) -> None:
        artifact, native = self.archive("php", ".zip")
        self.require_complete(artifact, native)

    def test_cli_directory_discovers_actual_executable(self) -> None:
        native, _ = self.layout("cli")
        self.require_complete(native=native)

    def test_cli_zip_discovers_actual_executable(self) -> None:
        artifact, native = self.archive("cli", ".zip")
        self.require_complete(artifact, native)

    def test_ffi_tar_reads_prefixed_lib_graph(self) -> None:
        artifact, native = self.archive("ffi", ".tar.gz")
        self.require_complete(artifact, native)

    def test_nif_tar_reads_versioned_root_graph(self) -> None:
        artifact, native = self.archive("nif", ".tar.gz")
        self.require_complete(artifact, native)

    def test_raw_directory_cannot_borrow_python_host_exemption(self) -> None:
        self.native.write_bytes(pe(["onnxruntime.dll", "python3.dll"]))
        self.qualify()
        with expected_error(ValueError, "python3.dll"):
            self.audit()

    def test_raw_provider_delay_import_cannot_borrow_python_host_exemption(self) -> None:
        (self.stage / "onnxruntime.dll").write_bytes(pe(delay=["python313.dll"]))
        self.qualify()
        with expected_error(ValueError, "python313.dll"):
            self.audit()

    def test_raw_missing_normal_crt_is_not_ort_presence_success(self) -> None:
        self.qualify()
        (self.stage / "vcruntime140.dll").unlink()
        with expected_error(ValueError, "vcruntime140.dll"):
            self.audit()

    def test_raw_missing_delay_crt_is_not_ort_presence_success(self) -> None:
        (self.stage / "onnxruntime.dll").write_bytes(pe(delay=["vcruntime140_1.dll"]))
        self.qualify()
        with expected_error(ValueError, "vcruntime140_1.dll"):
            self.audit()

    def test_raw_shipped_directml_is_still_forbidden(self) -> None:
        self.native.write_bytes(pe(["onnxruntime.dll", "DirectML.dll"]))
        (self.stage / "DirectML.dll").write_bytes(pe(["KERNEL32.dll"]))
        self.qualify(4)
        with expected_error(ValueError, "imports forbidden directml.dll"):
            self.audit()

    def test_raw_unrelated_search_directory_cannot_supply_import(self) -> None:
        self.qualify()
        elsewhere = self.stage / "elsewhere"
        elsewhere.mkdir()
        (self.stage / "vcruntime140.dll").rename(elsewhere / "vcruntime140.dll")
        with expected_error(ValueError, "missing wheel-local dependency vcruntime140.dll"):
            self.audit()

    def host(self) -> Path:
        directory = self.root / "host"
        directory.mkdir()
        host = directory / "php.exe"
        host.write_bytes(pe(["php8.dll", "KERNEL32.dll"]))
        (directory / "php8.dll").write_bytes(pe(["KERNEL32.dll"]))
        assert closure.PeImage(host.read_bytes()).imports() == ["php8.dll", "kernel32.dll"]
        return host

    def query(self, host: Path, **changes: object) -> mock._patch:
        info = {"version_id": 80330, "zts": 0, "debug": 0, "int_size": 8}
        info.update(changes)
        return mock.patch.object(
            closure.subprocess,
            "run",
            return_value=subprocess.CompletedProcess([str(host)], 0, stdout=json.dumps(info), stderr=""),
        )

    def test_verified_php_host_is_external_only_for_direct_php_root(self) -> None:
        self.native.write_bytes(pe(["onnxruntime.dll", "php8.dll"]))
        self.qualify()
        host = self.host()
        with self.query(host) as query:
            result = self.audit(php_host=host, php_version="8.3")
        assert result == {"files": 3, "edges": 4, "roots": 1}
        assert query.call_count == 1
        assert Path(query.call_args.args[0][0]) == host
        assert "PHP_ZTS" in query.call_args.args[0][-1]
        assert not (self.stage / "php8.dll").exists()

    def test_verified_php_host_does_not_exempt_transitive_provider_import(self) -> None:
        (self.stage / "onnxruntime.dll").write_bytes(pe(["php8.dll"]))
        self.qualify()
        host = self.host()
        with self.query(host), expected_error(ValueError, "php8.dll"):
            self.audit(php_host=host, php_version="8.3")

    def test_ffi_cannot_claim_php_host_profile(self) -> None:
        native, _ = self.layout("ffi")
        host = self.host()
        with self.query(host), expected_error(ValueError, "PHP host requires xberg.dll"):
            self.audit(native=native, php_host=host, php_version="8.3")

    def test_legacy_wheel_python_host_exemption_is_unchanged(self) -> None:
        self.native.rename(self.stage / "_native.pyd")
        (self.stage / "_native.pyd").write_bytes(pe(["onnxruntime.dll", "python313.dll"]))
        assert closure.audit_artifact(self.stage, "*.pyd") == {"files": 3, "edges": 4, "roots": 1}

    def test_cli_accepts_explicit_raw_mode_without_unknown_argument_failure(self) -> None:
        self.qualify()
        result = subprocess.run(
            [
                sys.executable,
                str(Path(closure.__file__)),
                "audit",
                "--artifact",
                str(self.stage),
                "--native-glob",
                "xberg.dll",
                "--artifact-kind",
                "raw",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result.stdout + result.stderr
        assert "Verified 3 PE files, 4 import edges, 1 native roots" in result.stdout


if __name__ == "__main__":
    unittest.main()
