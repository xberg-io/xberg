#!/usr/bin/env python3
"""Guard the per-target feature aggregates in `crates/xberg/Cargo.toml` against silent drift.

GH#1443. `windows-target` is a hand-maintained list that is supposed to carry everything
`full` does except the one capability with a real native blocker on Windows (`heic`, which
needs libheif via vcpkg). It had silently lost nine features -- `captioning`, `translation`,
`summarization-llm`, `ner-llm`, `redaction-ml`, `redaction-rehydrate`, `static-embeddings`,
`enrichment` and `otel` -- each of them a `#[cfg(feature = ...)]` gate over shipped code. Every
Windows wheel, gem and native artifact resolving through `windows-target` shipped without them.

`45bbd99088` put them back BY HAND. Nothing prevents the next hand-edit from dropping them
again: the only Windows CI job, `clippy-windows`, compiles `--features windows-target` and so
proves that the list builds, never that it is complete. `full` is exercised on Linux, and
nothing correlates the two. Adding a feature to `full` therefore silently re-opens GH#1443.
The same "keep in sync" pairs exist for `full`/`full-no-heic`, `formats`/`formats-no-heic`,
`windows-gnu-target` and `macos-intel-target`, all enforced by comment alone.

## Why "code-bearing"

A naive closure diff is useless here: it reports the aggregate NAMES (`formats`, `analysis`,
`services`) as missing from `windows-target`, because that list names their members
individually instead of the aggregate. Those names gate nothing -- grep confirms zero
`#[cfg(feature = "formats")]` (and likewise `analysis`, `services`, `full`, `full-no-heic`,
`formats-no-heic`) anywhere in `crates/xberg/src`. Excluding them by name would be an ad-hoc
patch that goes stale the moment another alias appears.

So this script compares only features that can actually cost you code: a feature is
CODE-BEARING when its own definition pulls at least one `dep:`, or its name appears in a
`#[cfg(feature = "...")]` in `crates/xberg/src`. A pure alias drops out, while every member it
forwards to is still compared -- so the rule cannot manufacture a false pass. Under it,
`full - windows-target` is exactly `{heic}`. ~keep

## What is deliberately NOT guarded

`android-target` and `no-ort-target` diverge from `full` by 23 and 39 code-bearing features
respectively, because they substitute a whole inference stack (`tract`) or drop ORT outright.
Encoding those sets would make this a change-detector -- it would fire on every legitimate
edit and get silenced -- rather than an invariant. They are listed here so their absence is a
recorded decision, not an oversight.

Exit codes: 0 = parity holds, 1 = a target lost a feature, 2 = malformed input.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

CORE_MANIFEST = Path("crates/xberg/Cargo.toml")
CORE_SRC = Path("crates/xberg/src")

# `heic` is the one capability with a genuine native blocker: it needs libheif, which is not on
# the standard GitHub windows-latest image and is not wired into the publish workflow's vcpkg
# step. Every other exclusion below is a platform toolchain limit, not a policy choice. ~keep
HEIC = frozenset({"heic"})

# candle's gemm-f16 matmul backend carries aarch64 inline asm requiring the `fullfp16` target
# feature, which the Linux aarch64 runner's rustc baseline lacks. The `*-no-heic` aggregates and
# the Intel-macOS target drop the candle VLM-OCR leaves for that reason. ~keep
CANDLE = frozenset(
    {
        "candle-ocr",
        "candle-trocr",
        "candle-glm-ocr",
        "candle-paddleocr-vl",
        "candle-deepseek-ocr",
    }
)

# The MinGW/Ruby target mirrors windows-target minus everything that reaches ONNX Runtime:
# pyke ships no gnu-ABI ORT prebuilt. ~keep
ORT_DEPENDENT = frozenset(
    {
        "auto-rotate",
        "embeddings",
        "late-interaction",
        "layout-detection",
        "ner-onnx",
        "onnx-runtime",
        "ort-bundled",
        "paddle-ocr-ort",
        "reranker",
        "sceptre-ocr-ort",
        "sparse-embeddings",
        "transcription",
    }
)

# (superset, subset, features the subset may legitimately lack, why).
# A subset missing anything NOT listed here is the GH#1443 failure and fails this check.
GUARDED_PAIRS = (
    ("full", "windows-target", HEIC, "libheif is not available on the Windows runner"),
    ("formats", "formats-no-heic", HEIC, "the whole point of the -no-heic variant"),
    ("full", "full-no-heic", HEIC | CANDLE, "libheif, plus candle needs aarch64 fullfp16"),
    ("full", "macos-intel-target", HEIC | CANDLE, "libheif, plus candle needs aarch64 fullfp16"),
    ("full", "windows-gnu-target", HEIC | CANDLE | ORT_DEPENDENT, "no gnu-ABI ONNX Runtime prebuilt"),
)

UNGUARDED = ("android-target", "no-ort-target")

_CFG_FEATURE = re.compile(r'feature\s*=\s*"([a-z0-9][a-z0-9._-]*)"')


def _fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)


def load_features(manifest: Path) -> dict[str, list[str]]:
    """Return the `[features]` table of a Cargo manifest."""
    with manifest.open("rb") as handle:
        config = tomllib.load(handle)
    features = config.get("features")
    if not isinstance(features, dict) or not features:
        raise ValueError(f"{manifest}: expected a non-empty [features] table")
    return {name: [e for e in entries if isinstance(e, str)] for name, entries in features.items()}


def cfg_referenced_features(src: Path) -> set[str]:
    """Every feature name appearing in a `feature = "..."` cfg predicate under `src`."""
    if not src.is_dir():
        raise ValueError(f"{src}: not a directory (run from the repository root)")
    found: set[str] = set()
    for path in src.rglob("*.rs"):
        found.update(_CFG_FEATURE.findall(path.read_text(encoding="utf-8", errors="replace")))
    return found


def closure(features: dict[str, list[str]], name: str) -> set[str]:
    """Transitively expand `name`, ignoring `dep:`, `crate/feat` and weak `crate?/feat` entries.

    Those three forms activate a dependency rather than another feature of this crate, so they
    are not comparable between aggregates and are not what GH#1443 was about.
    """
    seen: set[str] = set()
    stack = [name]
    while stack:
        for entry in features.get(stack.pop(), []):
            if entry.startswith("dep:") or "?" in entry or "/" in entry:
                continue
            if entry not in seen:
                seen.add(entry)
                stack.append(entry)
    return seen


def check(manifest: Path, src: Path) -> int:
    try:
        features = load_features(manifest)
        cfg_used = cfg_referenced_features(src)
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        _fail(str(error))
        return 2

    def code_bearing(name: str) -> bool:
        return any(e.startswith("dep:") for e in features.get(name, [])) or name in cfg_used

    missing_definitions = [name for pair in GUARDED_PAIRS for name in pair[:2] if name not in features]
    if missing_definitions:
        _fail(f"{manifest}: these aggregates are not defined: {', '.join(sorted(set(missing_definitions)))}")
        return 2

    failed = False
    for superset, subset, allowed, reason in GUARDED_PAIRS:
        expected = {f for f in closure(features, superset) if code_bearing(f)}
        actual = {f for f in closure(features, subset) if code_bearing(f)}

        missing = sorted(expected - actual - allowed)
        if missing:
            _fail(
                f"`{subset}` is missing {len(missing)} code-bearing feature(s) that `{superset}` "
                f"has: {', '.join(missing)}. Each one gates shipped code, so every artifact built "
                f"from `{subset}` silently lacks it -- this is GH#1443. Add them to "
                f"`{subset}` in {manifest}. The only features this target is allowed to lack are "
                f"{', '.join(sorted(allowed))} ({reason}); if one of the above genuinely cannot "
                f"build there either, extend that exclusion set with its own reason."
            )
            failed = True

        unnecessary = sorted(allowed - expected)
        if unnecessary:
            _fail(
                f"`{subset}`'s exclusion set names {', '.join(unnecessary)}, which `{superset}` "
                f"no longer contains. A stale exclusion widens the hole this check exists to "
                f"close: remove it from this script."
            )
            failed = True

    if failed:
        return 1

    print(
        f"OK: {len(GUARDED_PAIRS)} feature aggregate(s) at parity with their superset "
        f"({', '.join(pair[1] for pair in GUARDED_PAIRS)}). "
        f"Not guarded, by design: {', '.join(UNGUARDED)}."
    )
    return 0


def main() -> int:
    manifest = Path(sys.argv[1]) if len(sys.argv) > 1 else CORE_MANIFEST
    if not manifest.is_file():
        _fail(f"{manifest}: not found (run from the repository root)")
        return 2
    return check(manifest, CORE_SRC)


if __name__ == "__main__":
    sys.exit(main())
