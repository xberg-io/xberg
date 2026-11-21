"""Unit tests for the image notice verifier, driven by a fake Docker runner."""

from __future__ import annotations

import io
import os
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from typing import TYPE_CHECKING
from unittest import mock

from scripts.ci.docker import verify_image_notices as verifier

if TYPE_CHECKING:
    from collections.abc import Sequence

LICENSE = b"MIT License\n\nCopyright (c) Xberg\n"
THIRD_PARTY = b"Third-party notices\n\nDependency: example\n"


class FakeImage:
    """Serve notice bytes for `docker run --rm --entrypoint cat` calls."""

    def __init__(self, notices: dict[str, bytes]) -> None:
        self.notices = notices
        self.calls: list[list[str]] = []

    def __call__(self, arguments: Sequence[str]) -> subprocess.CompletedProcess[bytes]:
        call = list(arguments)
        self.calls.append(call)
        path = call[-1]
        name = path.rsplit("/", 1)[-1]
        if name not in self.notices:
            stderr = f"cat: {path}: No such file or directory".encode()
            return subprocess.CompletedProcess(call, 1, b"", stderr)
        return subprocess.CompletedProcess(call, 0, self.notices[name], b"")


class NoticeVerificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "LICENSE").write_bytes(LICENSE)
        (self.root / "THIRD_PARTY_LICENSES.md").write_bytes(THIRD_PARTY)
        self.notices = {"LICENSE": LICENSE, "THIRD_PARTY_LICENSES.md": THIRD_PARTY}

    def verify(self, image: verifier.DockerRunner) -> list[str]:
        return verifier.verify_image("xberg:core", self.root, image)

    def test_passes_when_both_notices_match_byte_for_byte(self) -> None:
        image = FakeImage(dict(self.notices))

        assert self.verify(image) == []
        assert image.calls == [
            [
                "run",
                "--rm",
                "--pull=never",
                "--entrypoint",
                "cat",
                "xberg:core",
                "/usr/local/share/licenses/xberg/LICENSE",
            ],
            [
                "run",
                "--rm",
                "--pull=never",
                "--entrypoint",
                "cat",
                "xberg:core",
                "/usr/local/share/licenses/xberg/THIRD_PARTY_LICENSES.md",
            ],
        ]

    def test_fails_when_a_notice_is_missing_from_the_image(self) -> None:
        image = FakeImage({"THIRD_PARTY_LICENSES.md": THIRD_PARTY})

        findings = self.verify(image)

        assert len(findings) == 1
        assert "xberg:core: LICENSE is missing from /usr/local/share/licenses/xberg" in findings[0]
        assert "No such file or directory" in findings[0]

    def test_fails_when_a_notice_is_truncated(self) -> None:
        truncated = LICENSE[: len(LICENSE) // 2]
        image = FakeImage({**self.notices, "LICENSE": truncated})

        findings = self.verify(image)

        assert findings == [
            f"xberg:core: LICENSE is truncated ({len(truncated)} of {len(LICENSE)} bytes)",
        ]

    def test_fails_when_a_notice_holds_stale_bytes_of_the_same_length(self) -> None:
        stale = LICENSE.replace(b"MIT", b"GPL", 1)
        image = FakeImage({**self.notices, "LICENSE": stale})

        findings = self.verify(image)

        assert findings == [
            f"xberg:core: LICENSE differs from the repository copy ({len(stale)} bytes vs {len(LICENSE)})",
        ]

    def test_fails_when_a_notice_is_empty(self) -> None:
        image = FakeImage({**self.notices, "THIRD_PARTY_LICENSES.md": b""})

        findings = self.verify(image)

        assert findings == [
            "xberg:core: THIRD_PARTY_LICENSES.md is empty",
        ]

    def test_reports_a_docker_that_cannot_be_executed(self) -> None:
        def missing_docker(_arguments: Sequence[str]) -> subprocess.CompletedProcess[bytes]:
            raise FileNotFoundError("docker is not on PATH")

        findings = verifier.verify_image("xberg:full", self.root, missing_docker)

        detail = "(docker could not be executed: docker is not on PATH)"
        assert findings == [
            f"xberg:full: LICENSE is missing from /usr/local/share/licenses/xberg {detail}",
            f"xberg:full: THIRD_PARTY_LICENSES.md is missing from /usr/local/share/licenses/xberg {detail}",
        ]

    def test_rejects_an_empty_canonical_notice(self) -> None:
        (self.root / "THIRD_PARTY_LICENSES.md").write_bytes(b"")

        try:
            verifier.verify_image("xberg:core", self.root, FakeImage(dict(self.notices)))
        except ValueError as error:
            if str(error) != "empty canonical notice: THIRD_PARTY_LICENSES.md":
                raise AssertionError(f"unexpected message: {error}") from error
        else:
            raise AssertionError("an empty canonical notice was accepted")


class CommandLineTests(unittest.TestCase):
    """Exercise the default runner through a fake `docker` executable on PATH."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "LICENSE").write_bytes(LICENSE)
        (self.root / "THIRD_PARTY_LICENSES.md").write_bytes(THIRD_PARTY)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        docker = self.bin / "docker"
        docker.write_text(
            "#!/bin/sh\n"
            "for last; do :; done\n"
            'file="$FAKE_IMAGE_DIR/$(basename "$last")"\n'
            'if [ ! -f "$file" ]; then echo "cat: $last: No such file or directory" >&2; exit 1; fi\n'
            'cat "$file"\n'
        )
        docker.chmod(0o755)
        self.image = self.root / "image"
        self.image.mkdir()
        (self.image / "LICENSE").write_bytes(LICENSE)
        (self.image / "THIRD_PARTY_LICENSES.md").write_bytes(THIRD_PARTY)

    def invoke(self, *arguments: str) -> tuple[int, str, str]:
        environment = {
            **os.environ,
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "FAKE_IMAGE_DIR": str(self.image),
        }
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.dict(os.environ, environment), redirect_stdout(stdout), redirect_stderr(stderr):
            status = verifier.main(["--image", "xberg:core", "--repository-root", str(self.root), *arguments])
        return status, stdout.getvalue(), stderr.getvalue()

    def test_reports_a_matching_image(self) -> None:
        status, stdout, stderr = self.invoke()

        assert status == 0
        assert stderr == ""
        assert stdout == (
            "xberg:core: packaged notices match the repository byte-for-byte: "
            f"LICENSE ({len(LICENSE)} bytes), THIRD_PARTY_LICENSES.md ({len(THIRD_PARTY)} bytes)\n"
        )

    def test_exits_non_zero_when_the_fake_image_lacks_a_notice(self) -> None:
        (self.image / "LICENSE").unlink()

        status, stdout, stderr = self.invoke()

        assert status == 1
        assert stdout == ""
        assert "error: xberg:core: LICENSE is missing from /usr/local/share/licenses/xberg" in stderr


if __name__ == "__main__":
    unittest.main()
