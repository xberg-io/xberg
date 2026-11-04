"""Exercise raw archive and PHP-host audit boundaries without executing Windows code."""

import io
import json
import stat
import subprocess
import tarfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

import test_windows_artifact_closure as fixtures
import windows_wheel_closure as closure
from test_windows_wheel_closure import expected_error, pe


class PhpHostBoundaries(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = fixtures.RawArtifactTests("runTest")
        self.fx.setUp()
        self.addCleanup(self.fx.doCleanups)
        self.fx.native.write_bytes(pe(["onnxruntime.dll", "php8.dll"]))
        self.fx.qualify()
        self.host = self.fx.host()

    def reject(self, message: str, **query: object) -> None:
        with self.fx.query(self.host, **query), expected_error(ValueError, message):
            self.fx.audit(php_host=self.host, php_version="8.3")

    def test_version_mismatch_is_rejected_before_host_exemption(self) -> None:
        self.reject("PHP host version", version_id=80401)

    def test_thread_safe_host_is_not_a_matching_nts_runtime(self) -> None:
        self.reject("PHP host thread safety", zts=1)

    def test_debug_host_is_not_a_matching_release_runtime(self) -> None:
        self.reject("PHP host debug", debug=1)

    def test_host_pointer_width_must_match_native_architecture(self) -> None:
        self.reject("PHP host pointer width", int_size=4)

    def test_boolean_query_fields_are_not_integer_host_evidence(self) -> None:
        self.reject("PHP host JSON fields", version_id=True)

    def test_host_executable_architecture_must_match_native_root(self) -> None:
        self.host.write_bytes(pe(["php8.dll"], machine=0xAA64))
        self.reject("PHP host PE architecture")

    def test_host_runtime_architecture_must_match_native_root(self) -> None:
        (self.host.parent / "php8.dll").write_bytes(pe(["KERNEL32.dll"], machine=0xAA64))
        self.reject("PHP host PE architecture")

    def test_missing_host_runtime_cannot_be_declared_external(self) -> None:
        (self.host.parent / "php8.dll").unlink()
        self.reject("PHP host runtime DLL missing")

    def test_host_executable_must_import_the_matching_abi(self) -> None:
        self.host.write_bytes(pe(["php8ts.dll"]))
        self.reject("PHP host must import php8.dll")

    def test_failed_host_command_is_rejected_even_with_valid_json(self) -> None:
        query = subprocess.CompletedProcess(
            [str(self.host)],
            23,
            stdout=json.dumps({"version_id": 80330, "zts": 0, "debug": 0, "int_size": 8}),
            stderr="host query failed",
        )
        with (
            mock.patch.object(closure.subprocess, "run", return_value=query),
            expected_error(ValueError, "PHP host command failed"),
        ):
            self.fx.audit(php_host=self.host, php_version="8.3")

    def test_host_command_json_must_be_a_complete_object(self) -> None:
        query = subprocess.CompletedProcess([str(self.host)], 0, stdout="[80330, 0, 0, 8]", stderr="")
        with (
            mock.patch.object(closure.subprocess, "run", return_value=query),
            expected_error(ValueError, "PHP host JSON fields"),
        ):
            self.fx.audit(php_host=self.host, php_version="8.3")

    def test_malformed_host_json_is_not_host_evidence(self) -> None:
        query = subprocess.CompletedProcess([str(self.host)], 0, stdout="not JSON", stderr="")
        with (
            mock.patch.object(closure.subprocess, "run", return_value=query),
            expected_error(ValueError, "PHP host JSON"),
        ):
            self.fx.audit(php_host=self.host, php_version="8.3")

    def test_direct_delay_php_import_has_the_same_verified_host_policy(self) -> None:
        self.fx.native.write_bytes(pe(["onnxruntime.dll", "KERNEL32.dll"], delay=["php8.dll"]))
        self.fx.qualify()
        with self.fx.query(self.host) as query:
            result = self.fx.audit(php_host=self.host, php_version="8.3")
        assert result == {"files": 3, "edges": 5, "roots": 1}
        assert query.call_count == 1
        assert not (self.fx.stage / "php8.dll").exists()

    def test_transitive_delay_php_import_is_not_a_host_dependency(self) -> None:
        self.fx.native.write_bytes(pe(["onnxruntime.dll", "KERNEL32.dll"]))
        (self.fx.stage / "onnxruntime.dll").write_bytes(pe(delay=["php8.dll"]))
        self.fx.qualify()
        with self.fx.query(self.host), expected_error(ValueError, "onnxruntime.dll missing"):
            self.fx.audit(php_host=self.host, php_version="8.3")

    def test_cli_root_cannot_request_php_host_authority(self) -> None:
        native, _ = self.fx.layout("cli")
        with self.fx.query(self.host), expected_error(ValueError, "PHP host requires xberg.dll"):
            self.fx.audit(native=native, php_host=self.host, php_version="8.3")

    def test_nif_root_cannot_request_php_host_authority(self) -> None:
        native, _ = self.fx.layout("nif")
        with self.fx.query(self.host), expected_error(ValueError, "PHP host requires xberg.dll"):
            self.fx.audit(native=native, php_host=self.host, php_version="8.3")


class RawMemberBoundaries(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = fixtures.RawArtifactTests("runTest")
        self.fx.setUp()
        self.addCleanup(self.fx.doCleanups)
        self.fx.qualify()

    def artifact(self, suffix: str, member: str, *, link: bool = False, duplicate: bool = False) -> Path:
        output = self.fx.root / ("unsafe" + suffix)
        entries = [(p.name, p.read_bytes()) for p in self.fx.stage.iterdir()]
        assert len(entries) == 3
        entries.append((member, b"payload"))
        if duplicate:
            entries.append((member.upper(), b"collision"))
        if suffix == ".zip":
            with zipfile.ZipFile(output, "w") as archive:
                for name, data in entries:
                    info = zipfile.ZipInfo(name)
                    # ZipInfo normalizes Windows separators before writing the filename.
                    info.filename = name
                    if link and name == member:
                        info.create_system = 3
                        info.external_attr = (stat.S_IFLNK | 0o777) << 16
                    archive.writestr(info, data)
            with zipfile.ZipFile(output) as archive:
                assert [info.orig_filename for info in archive.infolist()] == [name for name, _ in entries]
        else:
            with tarfile.open(output, "w:gz") as archive:
                for name, data in entries:
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    if link and name == member:
                        info.type = tarfile.SYMTYPE
                        info.linkname = "../outside.txt"
                        info.size = 0
                    archive.addfile(info, io.BytesIO(data) if info.isfile() else None)
            with tarfile.open(output) as archive:
                assert len(archive.getmembers()) == len(entries)
                assert member in archive.getnames()
        return output

    def reject(self, suffix: str, member: str, message: str, **options: bool) -> None:
        artifact = self.artifact(suffix, member, **options)
        with expected_error(ValueError, message):
            self.fx.audit(artifact)

    def test_zip_rejects_parent_escape_even_for_non_pe_members(self) -> None:
        self.reject(".zip", "../outside.txt", "unsafe artifact member")

    def test_zip_rejects_absolute_members(self) -> None:
        self.reject(".zip", "/outside.txt", "unsafe artifact member")

    def test_zip_rejects_windows_drive_members(self) -> None:
        self.reject(".zip", "C:/outside.txt", "unsafe artifact member")

    def test_zip_rejects_backslash_members(self) -> None:
        with mock.patch.object(zipfile.os, "sep", "\\"):
            assert zipfile.ZipInfo("payload\\outside.txt").filename == "payload/outside.txt"
            self.reject(".zip", "payload\\outside.txt", "unsafe artifact member")

    def test_zip_rejects_duplicate_casefolded_non_pe_members(self) -> None:
        self.reject(".zip", "notice.txt", "duplicate artifact member", duplicate=True)

    def test_zip_rejects_symbolic_links_without_following_them(self) -> None:
        self.reject(".zip", "linked.txt", "artifact links are forbidden", link=True)

    def test_tar_rejects_parent_escape_even_for_non_pe_members(self) -> None:
        self.reject(".tar.gz", "../outside.txt", "unsafe artifact member")

    def test_tar_rejects_duplicate_casefolded_non_pe_members(self) -> None:
        self.reject(".tar.gz", "notice.txt", "duplicate artifact member", duplicate=True)

    def test_tar_rejects_symbolic_links_without_following_them(self) -> None:
        self.reject(".tar.gz", "linked.txt", "artifact links are forbidden", link=True)

    def test_directory_rejects_external_pe_symlinks(self) -> None:
        target = self.fx.root / "outside.dll"
        target.write_bytes(pe(["KERNEL32.dll"]))
        link = self.fx.stage / "linked.dll"
        link.symlink_to(target)
        assert link.is_symlink() and link.resolve() == target.resolve()
        with expected_error(ValueError, "artifact links are forbidden"):
            self.fx.audit()


if __name__ == "__main__":
    unittest.main()
