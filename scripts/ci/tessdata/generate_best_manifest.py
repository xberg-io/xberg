#!/usr/bin/env python3
"""Write the tessdata_best download manifest the best Docker images verify against.

The Dockerfiles download the image's language set from tessdata_best at the revision pinned in
``crates/xberg/src/ocr/tessdata_download/pins.rs`` and check every file with ``sha256sum -c``.
This script derives both inputs from that table, so the images and the runtime downloader use
the same bytes:

- ``tessdata_best.revision``: the pinned upstream commit;
- ``tessdata_best.sha256``: one ``<sha256>  <file>`` line per language, plus the repository's
  ``LICENSE`` at that commit.

Usage:
    python3 scripts/ci/tessdata/generate_best_manifest.py          # rewrite both files
    python3 scripts/ci/tessdata/generate_best_manifest.py --check  # fail when they are stale
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parents[2]
PINS_PATH = ROOT / "crates/xberg/src/ocr/tessdata_download/pins.rs"
REVISION_PATH = SCRIPT_DIR / "tessdata_best.revision"
MANIFEST_PATH = SCRIPT_DIR / "tessdata_best.sha256"

# The thirteen languages the images install from Debian, plus the vertical Japanese and the Korean
# models, and equ (equation detection). The best directory is read-only and searched for best
# requests only, so osd and equ must be in it for orientation and equation detection to keep
# working; chi_sim and jpn_vert also serve the script retries. Shared by core-best and full-best.
IMAGE_LANGUAGES = (
    "ara",
    "chi_sim",
    "chi_tra",
    "deu",
    "eng",
    "equ",
    "fra",
    "hin",
    "ita",
    "jpn",
    "jpn_vert",
    "kor",
    "osd",
    "por",
    "rus",
    "spa",
)

# pins.rs pins only the models. The LICENSE digest is pinned here per revision, so moving the
# revision fails until the new commit's LICENSE has been read and pinned.
LICENSE_SHA256 = {
    "e12c65a915945e4c28e237a9b52bc4a8f39a0cec": "a6cba85bc92e0cff7a450b1d873c0eaa2e9fc96bf472df0247a26bec77bf3ff9",
}

REVISION_PATTERN = re.compile(r'TESSDATA_BEST_REVISION: &str = "([0-9a-f]{40})";')
ROW_PATTERN = re.compile(r'^\s+\("([a-z_]+)", "([0-9a-f]{64})", [0-9_]+\),$', re.MULTILINE)


def parse_pins(text: str) -> tuple[str, dict[str, str]]:
    """Return the tessdata_best revision and its language-to-SHA-256 table from pins.rs text."""
    revision = REVISION_PATTERN.search(text)
    if revision is None:
        raise ValueError("pins.rs has no TESSDATA_BEST_REVISION")
    if "TESSDATA_BEST_PACKS" not in text:
        raise ValueError("pins.rs has no TESSDATA_BEST_PACKS table")
    table = text.split("TESSDATA_BEST_PACKS", 1)[1].split("];", 1)[0]
    digests = dict(ROW_PATTERN.findall(table))
    if not digests:
        raise ValueError("TESSDATA_BEST_PACKS has no rows")
    return revision.group(1), digests


def render(pins_text: str) -> tuple[str, str]:
    """Return the revision file and the sha256sum manifest for the image language set."""
    revision, digests = parse_pins(pins_text)
    missing = [language for language in IMAGE_LANGUAGES if language not in digests]
    if missing:
        raise ValueError(f"TESSDATA_BEST_PACKS does not pin {', '.join(missing)}")
    if revision not in LICENSE_SHA256:
        raise ValueError(f"no LICENSE digest pinned for tessdata_best@{revision}; add it to LICENSE_SHA256")
    lines = [f"{digests[language]}  {language}.traineddata" for language in IMAGE_LANGUAGES]
    lines.append(f"{LICENSE_SHA256[revision]}  LICENSE")
    return f"{revision}\n", "\n".join(lines) + "\n"


def stale_files(pins_text: str, revision_text: str, manifest_text: str) -> list[str]:
    """Name each checked-in file whose content differs from what pins.rs implies."""
    expected_revision, expected_manifest = render(pins_text)
    stale = []
    if revision_text != expected_revision:
        stale.append(REVISION_PATH.name)
    if manifest_text != expected_manifest:
        stale.append(MANIFEST_PATH.name)
    return stale


def main(argv: list[str] | None = None) -> int:
    """Rewrite the manifest files, or with ``--check`` report the stale ones."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail instead of writing when a file is stale")
    args = parser.parse_args(argv)
    pins_text = PINS_PATH.read_text(encoding="utf-8")
    if args.check:
        stale = stale_files(
            pins_text,
            REVISION_PATH.read_text(encoding="utf-8") if REVISION_PATH.exists() else "",
            MANIFEST_PATH.read_text(encoding="utf-8") if MANIFEST_PATH.exists() else "",
        )
        for name in stale:
            print(f"error: scripts/ci/tessdata/{name} is stale; run {Path(__file__).name}", file=sys.stderr)
        if stale:
            return 1
        print(f"tessdata_best manifest matches pins.rs: {len(IMAGE_LANGUAGES)} languages and LICENSE")
        return 0
    revision_text, manifest_text = render(pins_text)
    REVISION_PATH.write_text(revision_text, encoding="utf-8")
    MANIFEST_PATH.write_text(manifest_text, encoding="utf-8")
    print(f"wrote {REVISION_PATH.name} and {MANIFEST_PATH.name} ({len(IMAGE_LANGUAGES)} languages and LICENSE)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
