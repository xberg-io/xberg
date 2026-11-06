"""Archive and loader boundary tests for the macOS Elixir release artifact."""

import gzip
import hashlib
import io
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import macos_elixir_archive as archive
import pytest

NIF = "libxberg_nif-v1.3.10-nif-2.16-aarch64-apple-darwin.so"
CODECS = {"libheif.1.dylib", "libde265.0.dylib", "libaom.3.dylib", "libx265.217.dylib"}
SYSTEM = "/usr/lib/libSystem.B.dylib"


class MacosElixirArchiveTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.tar = self.root / (NIF + ".tar.gz")
        self.extracted = self.root / "unpacked"
        self.extracted.mkdir()

    def write_members(self, members: list[tuple[str, bytes, bytes]]) -> None:
        with tarfile.open(self.tar, "w:gz", format=tarfile.USTAR_FORMAT) as output:
            for name, kind, data in members:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.size = len(data)
                output.addfile(member, io.BytesIO(data))

    def test_reads_the_exact_rustler_root(self) -> None:
        self.write_members([(NIF, tarfile.REGTYPE, b"nif")])
        archive.unpack(self.tar, self.extracted, {NIF})
        assert (self.extracted / NIF).read_bytes() == b"nif"

    def test_rejects_duplicate_root_before_overwriting(self) -> None:
        self.write_members([(NIF, tarfile.REGTYPE, b"original"), (NIF, tarfile.REGTYPE, b"replacement")])
        with pytest.raises(ValueError, match="duplicate"):
            archive.unpack(self.tar, self.extracted, {NIF})
        assert (self.extracted / NIF).read_bytes() == b"original"

    def test_rejects_traversal_nested_and_case_variant(self) -> None:
        for name in ["../" + NIF, "./" + NIF, "/tmp/" + NIF, NIF.upper()]:
            with self.subTest(name=name):
                self.write_members([(name, tarfile.REGTYPE, b"nif")])
                with pytest.raises(ValueError, match="unexpected member"):
                    archive.unpack(self.tar, self.extracted, {NIF})

    def test_rejects_links_directories_and_extended_headers(self) -> None:
        for kind in [tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.DIRTYPE, tarfile.XHDTYPE, tarfile.GNUTYPE_LONGNAME]:
            with self.subTest(kind=kind):
                self.write_members([(NIF, kind, b"")])
                with pytest.raises(ValueError, match="regular"):
                    archive.unpack(self.tar, self.extracted, {NIF})

    def test_allows_the_rustler_builders_mtime_only_pax_record(self) -> None:
        with tarfile.open(self.tar, "w:gz", format=tarfile.PAX_FORMAT) as output:
            member = tarfile.TarInfo(NIF)
            member.mtime = 1791940337.2343087
            member.size = 3
            output.addfile(member, io.BytesIO(b"nif"))
        archive.unpack(self.tar, self.extracted, {NIF})
        assert (self.extracted / NIF).read_bytes() == b"nif"
        with tempfile.TemporaryDirectory() as directory, pytest.raises(ValueError, match="regular"):
            archive.unpack(self.tar, Path(directory), {NIF}, canonical_modes=True)

    def test_rejects_pax_path_size_and_unrelated_metadata(self) -> None:
        for key, value in [("path", "../outside.so"), ("size", "3"), ("comment", "unrelated")]:
            with self.subTest(key=key), tarfile.open(self.tar, "w:gz", format=tarfile.PAX_FORMAT) as output:
                member = tarfile.TarInfo(NIF)
                member.size = 3
                member.pax_headers = {key: value}
                output.addfile(member, io.BytesIO(b"nif"))
            with self.subTest(key=key), pytest.raises(ValueError, match="PAX"):
                archive.unpack(self.tar, self.extracted, {NIF})

    def test_rejects_members_hidden_after_terminator(self) -> None:
        self.write_members([(NIF, tarfile.REGTYPE, b"nif")])
        first_tar = gzip.decompress(self.tar.read_bytes())
        self.tar.write_bytes(gzip.compress(first_tar + first_tar))
        with pytest.raises(ValueError, match="terminator"):
            archive.unpack(self.tar, self.extracted, {NIF})

    def test_rejects_nonzero_padding_and_truncated_payload(self) -> None:
        for data in [b"nif" + b"x" * 509, b"ni"]:
            with self.subTest(data_length=len(data)):
                member = tarfile.TarInfo(NIF)
                member.size = 3
                self.tar.write_bytes(gzip.compress(member.tobuf() + data))
                with tempfile.TemporaryDirectory() as directory, pytest.raises(ValueError, match=r"padding|truncated"):
                    archive.unpack(self.tar, Path(directory), {NIF})

    def test_rejects_excessive_declared_size_before_payload(self) -> None:
        member = tarfile.TarInfo(NIF)
        member.size = archive.MAX_UNCOMPRESSED + 1
        self.tar.write_bytes(gzip.compress(member.tobuf()))
        with pytest.raises(ValueError, match="size limit"):
            archive.unpack(self.tar, self.extracted, {NIF})

    def test_rejects_missing_required_member(self) -> None:
        self.write_members([(NIF, tarfile.REGTYPE, b"nif")])
        with pytest.raises(ValueError, match=r"missing.*libheif"):
            archive.unpack(self.tar, self.extracted, {NIF, "libheif.1.dylib"})

    def test_checksum_covers_final_compressed_bytes(self) -> None:
        self.write_members([(NIF, tarfile.REGTYPE, b"nif")])
        sidecar = self.root / (self.tar.name + ".sha256")
        sidecar.write_text(archive.checksum_row(self.tar), encoding="ascii")
        archive.verify_checksum(self.tar, sidecar)
        self.write_members([(NIF, tarfile.REGTYPE, b"different nif")])
        with pytest.raises(ValueError, match="checksum"):
            archive.verify_checksum(self.tar, sidecar)

    def test_system_paths_cannot_traverse_or_disguise_loader_search(self) -> None:
        assert archive.system_import(SYSTEM)
        assert archive.system_import("/System/Library/Frameworks/CoreML.framework/Versions/A/CoreML")
        for path in ["/usr/lib/../../opt/homebrew/x.dylib", "/usr/lib//x.dylib", "@rpath/x.dylib", "/System/./x"]:
            with self.subTest(path=path):
                assert not archive.system_import(path)

    def test_release_imports_require_existing_flat_siblings(self) -> None:
        assert archive.final_siblings(NIF, [SYSTEM, "@loader_path/libheif.1.dylib"], CODECS) == {"libheif.1.dylib"}
        for path in [
            "@rpath/libheif.1.dylib",
            "@loader_path/../libheif.1.dylib",
            "@loader_path/libunknown.dylib",
            "/opt/homebrew/lib/libheif.1.dylib",
        ]:
            with self.subTest(path=path), pytest.raises(ValueError, match="unsupported import"):
                archive.final_siblings(NIF, [SYSTEM, path], CODECS)

    def test_codec_graph_requires_each_encoder_dependency(self) -> None:
        edges = {NIF: {"libheif.1.dylib"}, "libheif.1.dylib": CODECS - {"libheif.1.dylib"}}
        edges.update({name: set() for name in CODECS if name != "libheif.1.dylib"})
        archive.verify_graph(NIF, edges)
        edges["libheif.1.dylib"].remove("libx265.217.dylib")
        with pytest.raises(ValueError, match="codec graph"):
            archive.verify_graph(NIF, edges)

    def test_source_import_rejects_same_basename_from_homebrew(self) -> None:
        library_dir = self.root / "lib"
        library_dir.mkdir()
        (library_dir / "libheif.1.dylib").write_bytes(b"codec")
        assert (
            archive.source_library(library_dir, str(library_dir / "libheif.1.dylib")) == library_dir / "libheif.1.dylib"
        )
        with pytest.raises(ValueError, match="outside the codec prefix"):
            archive.source_library(library_dir, "/opt/homebrew/lib/libheif.1.dylib")

    def test_arch_and_floor_check_refuses_newer_or_wrong_slice(self) -> None:
        for output in ["arm64 15.0\n", "x86_64 11.0\n"]:
            with (
                self.subTest(output=output),
                patch.object(archive, "tool", return_value=output),
                pytest.raises(ValueError, match=r"arm64.*11.0"),
            ):
                archive.verify_floor(self.root / NIF)

    def test_canonical_archive_rejects_unexpected_member_metadata(self) -> None:
        for field, value in [("uid", 1), ("gname", "custom"), ("mtime", 1), ("mode", 0o777)]:
            with self.subTest(field=field), tarfile.open(self.tar, "w:gz", format=tarfile.USTAR_FORMAT) as output:
                member = tarfile.TarInfo(NIF)
                member.size = 3
                member.mode = 0o755
                setattr(member, field, value)
                output.addfile(member, io.BytesIO(b"nif"))
            directory = self.extracted / field
            directory.mkdir()
            with self.subTest(field=field), pytest.raises(ValueError, match="unexpected metadata"):
                archive.unpack(self.tar, directory, {NIF}, canonical_modes=True)

    def test_validates_macho_type_before_running_external_tools(self) -> None:
        path = self.root / NIF
        for cpu, file_type, count in [
            (archive.CPU_TYPE_ARM64, 2, 1),
            (7, archive.MH_DYLIB, 1),
            (archive.CPU_TYPE_ARM64, archive.MH_DYLIB, archive.MAX_LOAD_COMMANDS + 1),
        ]:
            path.write_bytes(
                archive.MACH_HEADER.pack(archive.MH_MAGIC_64, cpu, 0, file_type, count, 8, 0, 0) + bytes(8)
            )
            with (
                self.subTest(cpu=cpu, file_type=file_type, count=count),
                pytest.raises(ValueError, match=r"Mach-O dylib|load command budget"),
            ):
                archive.verify_macho_header(path)

    def test_inspects_weak_imports_and_rejects_incomplete_tool_output(self) -> None:
        path = self.root / NIF
        identity = "/build/libxberg_nif.dylib"
        commands = "cmd LC_ID_DYLIB\ncmd LC_LOAD_DYLIB\ncmd LC_LOAD_WEAK_DYLIB\n"
        id_output = f"{path}:\n{identity}\n"
        imports = "".join(f" {name} (compatibility version 1.0.0)\n" for name in [identity, SYSTEM])
        weak = "@loader_path/libheif.1.dylib"
        weak_row = f" {weak} (compatibility version 1.0.0)\n"
        with patch.object(archive, "tool", side_effect=[id_output, commands, f"{path}:\n{imports}{weak_row}"]):
            assert archive.macho_imports(path) == [SYSTEM, weak]
        with (
            patch.object(archive, "tool", side_effect=[id_output, commands, f"{path}:\n{imports}"]),
            pytest.raises(ValueError, match="incomplete Mach-O imports"),
        ):
            archive.macho_imports(path)
        with (
            patch.object(archive, "tool", side_effect=[id_output, commands + "cmd LC_RPATH\n"]),
            pytest.raises(ValueError, match="unsupported loader search"),
        ):
            archive.macho_imports(path)

    def test_rejects_unrelocated_codec_identity(self) -> None:
        path = self.root / "libheif.1.dylib"
        with (
            patch.object(archive, "tool", return_value=f"{path}:\n/build/libheif.1.dylib\n"),
            pytest.raises(ValueError, match="unexpected relocated dylib identity"),
        ):
            archive.macho_imports(path, expected_id="@loader_path/libheif.1.dylib")

    def test_verification_compares_notice_bytes_to_the_independent_prefix(self) -> None:
        _, notices = self.notices()
        members = {NIF, *CODECS, *notices}
        stage = self.root / "stage"
        stage.mkdir()
        for name in members:
            (stage / name).write_bytes(notices.get(name, b"native"))
        (stage / "libaom-3.15.3-PATENTS").write_bytes(b"replaced")
        archive.pack(self.tar, stage, members)
        with pytest.raises(ValueError, match="archive source notice bytes differ"):
            archive.verify_release(self.tar, self.extracted, NIF, CODECS, notices)

    def test_no_overwrite_or_stray_checksum_on_failed_publication(self) -> None:
        output = self.root / "output"
        output.mkdir()
        self.tar.write_bytes(b"new archive")
        existing = output / self.tar.name
        existing.write_bytes(b"old archive")
        sidecar = self.root / (self.tar.name + ".sha256")
        sidecar.write_bytes(b"digest")
        with pytest.raises(FileExistsError):
            archive.publish_local(self.tar, sidecar, output)
        assert existing.read_bytes() == b"old archive"
        assert not (output / sidecar.name).exists()

    def notices(self) -> tuple[Path, dict[str, bytes]]:
        directory = self.root / "notices"
        directory.mkdir()
        names = {
            "sources.tsv": ("libheif 1.23.7", "libde265 1.1.3", "libaom 3.15.3", "x265 4.3"),
            "libheif-1.23.7-COPYING": (),
            "libde265-1.1.3-COPYING": (),
            "libde265-1.1.3-AUTHORS": (),
            "libde265-1.1.3-libde265-COPYING": (),
            "libaom-3.15.3-LICENSE": (),
            "libaom-3.15.3-PATENTS": (),
            "libaom-3.15.3-AUTHORS": (),
            "libaom-3.15.3-third_party-fastfeat-LICENSE": (),
            "libaom-3.15.3-third_party-libyuv-LICENSE": (),
            "libaom-3.15.3-third_party-vector-LICENSE": (),
            "x265_4.3-COPYING": (),
        }
        contents = {name: f"upstream {name}\n".encode("ascii") for name in names}
        urls = [
            "https://github.com/strukturag/libheif/releases/download/v1.23.7/libheif-1.23.7.tar.gz",
            "https://github.com/strukturag/libde265/releases/download/v1.1.3/libde265-1.1.3.tar.gz",
            "https://storage.googleapis.com/aom-releases/libaom-3.15.3.tar.gz",
            "https://github.com/Multicorewareinc/x265/releases/download/4.3/x265_4.3.tar.gz",
        ]
        contents["sources.tsv"] = "".join(
            f"{release}\t{'a' * 64}\t{url}\n" for release, url in zip(names["sources.tsv"], urls, strict=True)
        ).encode("ascii")
        contents["notices.sha256"] = "".join(
            f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in contents.items()
        ).encode("ascii")
        for name, data in contents.items():
            (directory / name).write_bytes(data)
        return directory, contents

    def test_requires_encoder_build_receipt_matching_notice_source_versions(self) -> None:
        _, notices = self.notices()
        marker = self.root / ".xberg-heif-encoder-profile"
        receipt = self.root / ".xberg-heif-versions"
        marker.touch()
        expected = (
            "libheif=1.23.7 libde265=1.1.3 libaom=3.15.3 target=11.0 arch=arm64 profile=elixir-encoders x265=4.3\n"
        )
        receipt.write_text(expected, encoding="ascii")
        archive.verify_source_receipt(self.root, notices["sources.tsv"])
        receipt.write_text(expected.replace("3.15.3", "3.15.2"), encoding="ascii")
        with pytest.raises(ValueError, match="source codec build receipt"):
            archive.verify_source_receipt(self.root, notices["sources.tsv"])
        marker.unlink()
        with pytest.raises(ValueError, match="source codec profile marker"):
            archive.verify_source_receipt(self.root, notices["sources.tsv"])

    def test_source_notice_manifest_requires_exact_bytes_and_members(self) -> None:
        directory, contents = self.notices()
        assert archive.read_notices(directory) == contents
        path = directory / "libaom-3.15.3-PATENTS"
        path.write_bytes(b"changed")
        with pytest.raises(ValueError, match="notice checksum"):
            archive.read_notices(directory)
        path.write_bytes(contents[path.name])
        path.unlink()
        with pytest.raises(ValueError, match="notice inventory"):
            archive.read_notices(directory)
        path.write_bytes(contents[path.name])
        (directory / "extra.txt").write_bytes(b"unexpected")
        with pytest.raises(ValueError, match="notice inventory"):
            archive.read_notices(directory)

    def test_extra_manifest_row_rejected_even_if_file_exists(self) -> None:
        directory, contents = self.notices()
        (directory / "notices.sha256").write_bytes(contents["notices.sha256"] + b"a" * 64 + b"  extra.txt\n")
        (directory / "extra.txt").write_bytes(b"unexpected")
        with pytest.raises(ValueError, match="notice inventory"):
            archive.read_notices(directory)
