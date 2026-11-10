"""Exercise the changelog gate through its public command with real files."""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-changelog-sync.py").resolve()
DOCS = Path("docs-site/src/content/docs")
CURRENT = b"## [Unreleased]\n\nNext.\n\n## [1.3.0] - 2026-09-28\n\nCurrent.\n\n"
HISTORY = b"## [1.2.9] - 2026-09-24\n\nOlder [link](https://example.org).\n\n## [1.0.0]\n\nFirst.\n"


class ChangelogSyncTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / DOCS).mkdir(parents=True)
        for name, sections in (("CHANGELOG.md", CURRENT), ("CHANGELOG-1.0-1.2.md", HISTORY)):
            self.path(name).write_bytes(b"# Changelog\n\n" + sections)
            self.path(name, docs=True).write_bytes(b"---\ntitle: Changelog\n---\n\n" + sections)

    def path(self, name: str = "CHANGELOG-1.0-1.2.md", *, docs: bool = False) -> Path:
        return self.root / DOCS / name.lower() if docs else self.root / name

    def run_gate(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args], cwd=self.root, text=True, capture_output=True, check=False
        )

    def test_should_check_both_current_and_archived_sections(self) -> None:
        result = self.run_gate()
        assert result.returncode == 0, result.stderr
        assert result.stdout.count("all 2 release sections") == 2, result.stdout

    def test_should_reject_missing_archive_on_either_side(self) -> None:
        for docs in (False, True):
            with self.subTest(docs=docs):
                path = self.path(docs=docs)
                original = path.read_bytes()
                path.unlink()
                result = self.run_gate()
                assert result.returncode == 2, result.stdout
                assert str(path.relative_to(self.root)) in result.stderr
                path.write_bytes(original)

    def test_should_reject_archive_body_date_order_and_missing_release_drift(self) -> None:
        for changed in (
            HISTORY.replace(b"Older", b"Altered"),
            HISTORY.replace(b"2026-09-24", b"2026-09-25"),
            b"## [1.0.0]\n\nFirst.\n\n## [1.2.9] - 2026-09-24\n\nOlder.\n",
            HISTORY[HISTORY.index(b"## [1.0.0]") :],
            HISTORY + b"\n## [0.9.0]\n\nExtra.\n",
            HISTORY + HISTORY,
            b"",
            HISTORY.rstrip(b"\n"),
            HISTORY.replace(b"\n", b"\r\n"),
        ):
            with self.subTest(changed=changed):
                self.path(docs=True).write_bytes(b"Docs preamble\n\n" + changed)
                result = self.run_gate()
                assert result.returncode == 1, result.stdout
                assert "ERROR:" in result.stderr

    def test_should_keep_current_release_drift_detection(self) -> None:
        self.path("CHANGELOG.md", docs=True).write_bytes(CURRENT.replace(b"Current", b"Wrong"))
        result = self.run_gate()
        assert result.returncode == 1
        assert "[1.3.0] section differs" in result.stderr

    def test_should_sync_both_pairs_exactly_preserving_preambles_and_idempotence(self) -> None:
        for name in ("CHANGELOG.md", "CHANGELOG-1.0-1.2.md"):
            self.path(name, docs=True).write_bytes(b"Unique preamble\r\n\r\n## [Unreleased]\n\nStale.\n")
        result = self.run_gate("--fix")
        assert result.returncode == 0, result.stderr
        expected = [b"Unique preamble\r\n\r\n" + body for body in (CURRENT, HISTORY)]
        paths = [self.path(name, docs=True) for name in ("CHANGELOG.md", "CHANGELOG-1.0-1.2.md")]
        assert [path.read_bytes() for path in paths] == expected
        assert self.run_gate("--fix").returncode == 0
        assert [path.read_bytes() for path in paths] == expected

    def test_should_fail_clearly_when_fix_has_no_release_heading(self) -> None:
        self.path().write_bytes(b"# Empty archive\n")
        result = self.run_gate("--fix")
        assert result.returncode == 1
        assert "no release heading" in result.stderr
        assert "Traceback" not in result.stderr


class ArchiveNavigationTests(unittest.TestCase):
    def test_should_pin_the_docs_archive_route_used_by_its_links(self) -> None:
        archive = SCRIPT.parents[2] / DOCS / "changelog-1.0-1.2.md"
        frontmatter = archive.read_text().split("---", 2)[1]
        assert "slug: changelog-1.0-1.2" in frontmatter.splitlines()

    def test_should_preserve_full_dated_heading_ids_and_archive_destinations(self) -> None:
        root = SCRIPT.parents[2]
        for current, archive, destination in (
            (root / "CHANGELOG.md", root / "CHANGELOG-1.0-1.2.md", "CHANGELOG-1.0-1.2.md"),
            (root / DOCS / "changelog.md", root / DOCS / "changelog-1.0-1.2.md", "/changelog-1.0-1.2/"),
        ):
            with self.subTest(current=current):
                headings = re.findall(r"^## (\[([\d.]+)\](.*))$", archive.read_text(), re.MULTILINE)
                assert headings, "the archive must contain release headings"
                assert headings[0] == ("[1.2.9] - 2026-09-24", "1.2.9", " - 2026-09-24")
                index = re.findall(r"^### .*", current.read_text(), re.MULTILINE)[: len(headings)]
                expected = []
                for title, version, date in headings:
                    fragment = re.sub(r"[^a-z0-9 -]", "", title.lower()).replace(" ", "-")
                    expected.append(f"### [{version}]({destination}#{fragment}){date}")
                assert index == expected, (current, index, expected)


if __name__ == "__main__":
    unittest.main()
