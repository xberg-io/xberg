---
type: Decision
title: Extraction Measurement Hygiene
paths:
  - "tools/benchmark-harness/**"
  - "tools/ocr-measure/**"
  - "tools/perf/**"
  - "crates/xberg/src/ocr/**"
  - "crates/xberg-tesseract/**"
  - "crates/xberg-paddle-ocr/**"
  - "crates/xberg-candle-ocr/**"
x-ai-rulez:
  kind: rule
  id: extraction-measurement-hygiene
  metadata:
    priority: high
---

Rules for any A/B or sweep that measures xberg extraction output. Each of these has silently
invalidated a run.

**Isolate the cache per invocation, and guard against an empty path.**

```sh
cd=$(mktemp -d) || exit 90
[ -n "$cd" ] && [ -d "$cd" ] || exit 90   # an empty XBERG_CACHE_DIR silently shares the global cache
XBERG_CACHE_DIR="$cd" <bin> extract -c <cfg> --ocr-no-cache true --no-cache true <file>
rm -rf "$cd"
```

An unset or empty `XBERG_CACHE_DIR` falls back to the user cache, which can be gigabytes of results
from earlier runs. The failure is invisible: the sweep completes and reports clean numbers.

**`--ocr-no-cache` is a no-op unless `ocr.tesseract_config` is set in the config**
(`overrides.rs`). Every sweep config must carry an explicit `[ocr.tesseract_config]` block with its
own nested `language`, or configs score each other's cached output. The nested `language` is also
required in its own right: the outer `OcrConfig.language` is read *only* when `tesseract_config` is
`None`, so supplying the table without a language silently falls back to English.

**Cross-binary cache leakage is already prevented** — the cache key folds in `XBERG_BUILD_ID`, a
per-build git identifier, so entries written by one build are unreachable from another. Do not
assume it; if a run depends on it, prove it in both directions, and first prove the cache was live
at all (a second run must be measurably faster and write no new entries). "No leak" and "the cache
never ran" render identically.

**Read the payload at the right level.** The CLI's JSON output nests the document under `result`
(`{extraction_time_ms, peak_memory_bytes, result:{content, extraction_method, …}}`). Reading
`extraction_method` off the top level yields `None`, which looks exactly like a failed extraction.
`extraction_method` is absent entirely for non-PDF, non-image formats.

**Timings from a shared machine are upper bounds only.** Peer agents compile and sweep on the same
box; report elapsed time as contended, or measure it when the box is quiet. Peak RSS is per-process
and is safe to compare.
