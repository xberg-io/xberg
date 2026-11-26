#!/usr/bin/env python3
"""Generate a pinned tessdata table (language, SHA-256, byte size) for crates/xberg.

Every file is downloaded at the given revision, checked against the size and Git blob SHA-1
that the GitHub tree API reports for that revision, and hashed with SHA-256. The output is
the body of a ``TESSDATA_*_PACKS`` table in ``crates/xberg/src/ocr/tessdata_download/pins.rs``.
The language set is the one already pinned there, so both model sets cover the same codes.

Usage:
    python3 -I scripts/generate_tessdata_pins.py --repo tessdata_best \
        --revision e12c65a915945e4c28e237a9b52bc4a8f39a0cec > /tmp/best-pins.rs

``frk`` is fetched from ``deu_latf``: upstream renamed the Fraktur model, keeps ``frk`` only as a
Git symlink in tessdata_fast and dropped it from tessdata_best. The runtime downloader applies the
same mapping.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

PINS_PATH = Path(__file__).resolve().parent.parent / "crates/xberg/src/ocr/tessdata_download/pins.rs"
SOURCE_ALIASES = {"frk": "deu_latf"}
TRAINEDDATA_SUFFIX = ".traineddata"
WORKERS = 8
TIMEOUT_SECONDS = 600


def pinned_languages() -> list[str]:
    """Return the language codes of the tessdata_fast table."""
    text = PINS_PATH.read_text(encoding="utf-8")
    table = text.split("TESSDATA_FAST_PACKS", 1)[1].split("];", 1)[0]
    return re.findall(r'^\s+\("([a-z_]+)"', table, flags=re.MULTILINE)


def fetch(url: str) -> bytes:
    """Download ``url`` and return its body."""
    request = urllib.request.Request(url, headers={"User-Agent": "tessdata-pin-generator"})  # noqa: S310
    with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:  # noqa: S310
        return response.read()


def tree_blobs(repo: str, revision: str) -> dict[str, tuple[str, int]]:
    """Map each top-level ``.traineddata`` stem to its Git blob SHA-1 and size."""
    tree = json.loads(fetch(f"https://api.github.com/repos/tesseract-ocr/{repo}/git/trees/{revision}"))
    if tree.get("truncated"):
        msg = "GitHub tree listing is truncated"
        raise SystemExit(msg)
    return {
        entry["path"].removesuffix(TRAINEDDATA_SUFFIX): (entry["sha"], entry["size"])
        for entry in tree["tree"]
        if entry["path"].endswith(TRAINEDDATA_SUFFIX) and entry["mode"] == "100644"
    }


def pin(repo: str, revision: str, lang: str, blobs: dict[str, tuple[str, int]]) -> str:
    """Download one model, verify it against its Git blob and format its table row."""
    source = SOURCE_ALIASES.get(lang, lang) if lang not in blobs else lang
    if source not in blobs:
        msg = f"{repo}@{revision} has no {lang}{TRAINEDDATA_SUFFIX}"
        raise SystemExit(msg)
    blob_sha, blob_size = blobs[source]
    data = fetch(f"https://raw.githubusercontent.com/tesseract-ocr/{repo}/{revision}/{source}{TRAINEDDATA_SUFFIX}")
    git_sha = hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()  # noqa: S324
    if len(data) != blob_size or git_sha != blob_sha:
        msg = f"{source}: got {len(data)} bytes / blob {git_sha}, tree says {blob_size} / {blob_sha}"
        raise SystemExit(msg)
    return f'    ("{lang}", "{hashlib.sha256(data).hexdigest()}", {len(data):_}),'


def main() -> int:
    """Print the pinned table rows for ``--repo`` at ``--revision``."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True, choices=["tessdata_fast", "tessdata_best"])
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()

    languages = pinned_languages()
    blobs = tree_blobs(args.repo, args.revision)
    with ThreadPoolExecutor(WORKERS) as pool:
        rows = list(pool.map(lambda lang: pin(args.repo, args.revision, lang, blobs), languages))
    sys.stdout.write("\n".join(rows) + "\n")
    sys.stderr.write(f"pinned {len(rows)} of {len(languages)} languages from {args.repo}@{args.revision}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
