#!/usr/bin/env python3
"""Guard the docs-site changelog against drifting behind the repo CHANGELOG.

`docs-site/src/content/docs/changelog.md` is a hand-maintained copy of
`CHANGELOG.md` with a Starlight frontmatter block bolted on. Nothing generated
it, nothing synced it, and nothing checked it -- so it silently fell a full
release behind: the repo carried 1.1.0 (2026-08-07) while the published docs
still topped out at 1.0.14.

This asserts the two agree on the part they share:

  1. every `## [x.y.z]` section heading in CHANGELOG.md is present in the docs
     copy, in the same order -- so a new release cannot ship with the docs
     silently stale, and
  2. each shared section's body is byte-identical -- so an edit to one side
     cannot quietly diverge from the other.

Checking only (1) would pass on a copy whose bodies had been reworded; checking
only (2) would pass on a copy that is simply missing the newest release. The
drift this exists to prevent lived in the gap between them.

Frontmatter and the intro preamble differ by design and are not compared. The
same check and --fix cover the 1.0-1.2 archive; release bodies retain their exact
line endings and final newline.

Run from the repository root:

    python3 scripts/ci/check-changelog-sync.py
    python3 scripts/ci/check-changelog-sync.py --fix

Exits 0 when in sync, 1 on drift, 2 when a file is missing.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_CHANGELOG = Path("CHANGELOG.md")
DOCS_CHANGELOG = Path("docs-site/src/content/docs/changelog.md")
CHANGELOG_PAIRS = (
    (REPO_CHANGELOG, DOCS_CHANGELOG),
    (Path("CHANGELOG-1.0-1.2.md"), DOCS_CHANGELOG.with_name("changelog-1.0-1.2.md")),
)

# `Unreleased` counts as a section: it is published to the docs site like any other, and
# leaving it out meant `sync` started copying at the first numeric heading, so the docs
# kept a stale Unreleased section that neither the check nor --fix ever touched. ~keep
VERSION_HEADING = re.compile(r"^## \[?(\d+\.\d+\.\d+|Unreleased)\]?")

# How many differing lines to show before truncating a body mismatch report.
MAX_DIFF_LINES = 12


def _fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)


def _sections(lines: list[str]) -> dict[str, list[str]]:
    """Split a changelog into {version: body_lines}, ignoring anything before the first version."""
    sections: dict[str, list[str]] = {}
    current: str | None = None
    for line in lines:
        match = VERSION_HEADING.match(line)
        if match:
            current = match.group(1)
            # The heading line goes INTO the body, not just its captured version. Keying on the
            # version alone meant the rest of the line -- the release date -- was never compared,
            # so a wrong or missing date passed as "byte-identical". ~keep
            sections[current] = [line]
        elif current is not None:
            sections[current].append(line)
    return sections


def _versions_in_order(lines: list[str]) -> list[str]:
    return [m.group(1) for m in (VERSION_HEADING.match(line) for line in lines) if m]


def _read_lines(path: Path) -> list[str]:
    return path.read_bytes().decode("utf-8").splitlines(keepends=True)


def check(repo_path: Path, docs_path: Path) -> int:
    repo_lines = _read_lines(repo_path)
    docs_lines = _read_lines(docs_path)

    repo_versions = _versions_in_order(repo_lines)
    docs_versions = _versions_in_order(docs_lines)

    if not repo_versions:
        _fail(f"{repo_path}: no '## [x.y.z]' version headings found")
        return 1

    if len(repo_versions) != len(set(repo_versions)) or repo_versions != docs_versions:
        _fail(
            f"{docs_path}: release versions must match {repo_path} exactly, once each and in order\n"
            f"       docs: {docs_versions}\n       repo: {repo_versions}"
        )
        return 1

    repo_sections = _sections(repo_lines)
    docs_sections = _sections(docs_lines)
    for version in repo_versions:
        if repo_sections[version] != docs_sections[version]:
            differing = [
                f"         repo: {r}\n         docs: {d}"
                # strict=False on purpose: a length mismatch IS one of the drifts being
                # reported, so it must not raise before the report is printed.
                for r, d in zip(repo_sections[version], docs_sections[version], strict=False)
                if r != d
            ][:MAX_DIFF_LINES]
            _fail(f"{docs_path}: the [{version}] section differs from {repo_path}\n" + "\n".join(differing))
            return 1

    print(f"OK: {docs_path} carries all {len(repo_versions)} release sections from {repo_path}, byte-identical")
    return 0


def sync(repo_path: Path, docs_path: Path) -> None:
    """Replace the docs' release sections, Unreleased included, with the canonical repo ones."""
    repo_lines = _read_lines(repo_path)
    docs_lines = _read_lines(docs_path)
    for path, lines in ((repo_path, repo_lines), (docs_path, docs_lines)):
        if not _versions_in_order(lines):
            raise ValueError(f"{path}: no release heading found; cannot sync")
    repo_start = next(index for index, line in enumerate(repo_lines) if VERSION_HEADING.match(line))
    docs_start = next(index for index, line in enumerate(docs_lines) if VERSION_HEADING.match(line))
    docs_path.write_bytes("".join(docs_lines[:docs_start] + repo_lines[repo_start:]).encode("utf-8"))


def main() -> int:
    if sys.argv[1:] not in ([], ["--fix"]):
        _fail("usage: check-changelog-sync.py [--fix]")
        return 2
    for path in (path for pair in CHANGELOG_PAIRS for path in pair):
        if not path.is_file():
            _fail(f"{path}: not found (run from the repository root)")
            return 2
    try:
        if sys.argv[1:] == ["--fix"]:
            for repo_path, docs_path in CHANGELOG_PAIRS:
                sync(repo_path, docs_path)
        return max(check(repo_path, docs_path) for repo_path, docs_path in CHANGELOG_PAIRS)
    except ValueError as error:
        _fail(f"syncing changelog: {error}")
        return 1
    except OSError as error:
        _fail(f"reading or syncing changelog: {error}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
