#!/usr/bin/env bash
#
# Local head-to-head PDF benchmark: Xberg (heuristics + routed layout) vs
# LiteParse (and Docling, if installed). Establishes the Phase-0 baseline —
# per-document quality with a gap report — that every later change is measured
# against.
#
# It runs two modes:
#   single-file : per-document quality (TF1/SF1/combined) + cold start + single
#                 file throughput. This feeds the gap report.
#   batch       : native batch throughput. Xberg, Docling, and LiteParse require an
#                 explicitly homogeneous OCR cohort; mixed cohorts are rejected
#                 because sequential fallback is not comparable batch throughput.
#
# Environment overrides:
#   HEURISTIC_FIXTURES OCR-disabled, non-OCR-only fixture cohort (default: unset)
#   OCR_FIXTURES OCR-enabled, OCR-required fixture cohort (default: unset)
#   OUT          output dir   (default: tools/benchmark-harness/results/local)
#   FRAMEWORKS   single-file frameworks (default: xberg baseline+layout, liteparse)
#   ITERATIONS   iterations per doc (default: 1)
#   TIMEOUT      per-extraction timeout seconds (default: 300)
#   SHARD        run a subset, e.g. "1/60" for a quick smoke run (default: none)
#   BATCH_HEURISTIC_FIXTURES non-OCR batch cohort (default: unset)
#   BATCH_OCR_FIXTURES all-OCR batch cohort (default: unset)
#   BATCH_FRAMEWORKS native-batch frameworks (default: native-capable subset of FRAMEWORKS)
#   BATCH_WORKERS framework worker limit where its native API exposes one (default: 4)
#   SKIP_BUILD   set to 1 to skip the cargo builds (default: build) ~keep
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$REPO_ROOT"

HEURISTIC_FIXTURES="${HEURISTIC_FIXTURES:-}"
OCR_FIXTURES="${OCR_FIXTURES:-}"
OUT="${OUT:-tools/benchmark-harness/results/local}"
FRAMEWORKS_EXPLICIT=0
if [ "${FRAMEWORKS+x}" = x ]; then
  FRAMEWORKS_EXPLICIT=1
fi
FRAMEWORKS="${FRAMEWORKS:-xberg-markdown-baseline,xberg-markdown-layout,liteparse}"
ITERATIONS="${ITERATIONS:-1}"
TIMEOUT="${TIMEOUT:-300}"
SHARD="${SHARD:-}"
BATCH_HEURISTIC_FIXTURES="${BATCH_HEURISTIC_FIXTURES:-}"
BATCH_OCR_FIXTURES="${BATCH_OCR_FIXTURES:-}"
BATCH_FRAMEWORKS_EXPLICIT=0
if [ "${BATCH_FRAMEWORKS+x}" = x ]; then
  BATCH_FRAMEWORKS_EXPLICIT=1
fi
BATCH_FRAMEWORKS="${BATCH_FRAMEWORKS:-}"
BATCH_WORKERS="${BATCH_WORKERS:-4}"

source "$REPO_ROOT/tools/benchmark-harness/scripts/bench_local_frameworks.sh"

if [ "$BATCH_FRAMEWORKS_EXPLICIT" = 1 ]; then
  BATCH_FRAMEWORKS="$(validate_native_batch_frameworks "$BATCH_FRAMEWORKS")"
fi

if ! command -v lit >/dev/null 2>&1; then
  for cand in /tmp/liteparse/target/release ../liteparse/target/release; do
    if [ -x "$cand/lit" ]; then
      export PATH="$cand:$PATH"
      break
    fi
  done
fi
if command -v lit >/dev/null 2>&1; then
  echo "[bench:local] lit: $(command -v lit) ($(lit --version 2>/dev/null))"
else
  echo "[bench:local] WARN: lit not found — liteparse rows will be skipped." >&2
fi

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  echo "[bench:local] Building xberg CLI (release, --features all)…"
  cargo build --locked --release -p xberg-cli --features all
  echo "[bench:local] Building benchmark harness (release)…"
  cargo build --locked --release -p benchmark-harness
fi
HARNESS=./target/release/benchmark-harness

# 3. Resolve Docling once, outside measured extraction, to the exact direct
#    interpreter that the Rust adapter will use. ~keep
resolve_docling_python() {
  local candidate resolved
  if [ -n "${XBERG_BENCH_PYTHON:-}" ]; then
    candidate="$XBERG_BENCH_PYTHON"
    resolved="$(command -v "$candidate" 2>/dev/null || true)"
    if [ -z "$resolved" ] || ! "$resolved" -c "import docling" >/dev/null 2>&1; then
      echo "[bench:local] XBERG_BENCH_PYTHON=$candidate cannot import docling." >&2
      return 1
    fi
    "$resolved" -c "import sys; print(sys.executable)"
    return
  fi

  for candidate in \
    "${VIRTUAL_ENV:+$VIRTUAL_ENV/bin/python}" \
    "${VIRTUAL_ENV:+$VIRTUAL_ENV/Scripts/python.exe}" \
    .venv/bin/python \
    .venv/Scripts/python.exe \
    python3 \
    python; do
    [ -n "$candidate" ] || continue
    resolved="$(command -v "$candidate" 2>/dev/null || true)"
    if [ -n "$resolved" ] && "$resolved" -c "import docling" >/dev/null 2>&1; then
      "$resolved" -c "import sys; print(sys.executable)"
      return
    fi
  done

  if command -v uv >/dev/null 2>&1; then
    uv run --locked --no-sync --group bench-docling python -c \
      "import docling, sys; print(sys.executable)"
    return
  fi
  return 1
}

WANT_DOCLING=0
DOCLING_EXPLICIT=0
if [ "$FRAMEWORKS_EXPLICIT" = 0 ]; then
  WANT_DOCLING=1
fi
if docling_is_explicitly_requested "$FRAMEWORKS" "$BATCH_FRAMEWORKS"; then
  WANT_DOCLING=1
  DOCLING_EXPLICIT=1
fi

if [ "$WANT_DOCLING" = 0 ]; then
  :
elif BENCH_PYTHON="$(resolve_docling_python)"; then
  export XBERG_BENCH_PYTHON="$BENCH_PYTHON"
  if [ "$FRAMEWORKS_EXPLICIT" = 0 ]; then
    echo "[bench:local] docling detected — including it."
    FRAMEWORKS="$FRAMEWORKS,docling"
  fi
else
  if [ "$DOCLING_EXPLICIT" = 1 ]; then
    echo "[bench:local] docling was requested but no prepared interpreter can import it." >&2
    exit 1
  fi
  echo "[bench:local] docling not installed — skipping (install with: uv sync --locked --group bench-docling)."
fi

if [ "$BATCH_FRAMEWORKS_EXPLICIT" = 0 ]; then
  BATCH_FRAMEWORKS="$(native_batch_frameworks "$FRAMEWORKS")"
fi

SHARD_ARGS=()
[ -n "$SHARD" ] && SHARD_ARGS=(--shard "$SHARD")

validate_ocr_cohort() {
  local cohort="$1"
  local expected="$2"
  python3 - "$cohort" "$expected" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
expected = sys.argv[2] == "true"
image_types = {"png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "jp2", "jpx", "jpm", "mj2"}
fixture_paths = sorted(root.rglob("*.json")) if root.is_dir() else [root]
bad = []
for path in fixture_paths:
    try:
        fixture = json.loads(path.read_text(encoding="utf-8"))
        metadata_value = fixture.get("metadata", {}).get("requires_ocr")
        if isinstance(metadata_value, bool):
            requires_ocr = metadata_value
        else:
            file_type = str(fixture.get("file_type", "")).lower()
            document_type = pathlib.Path(str(fixture.get("document", ""))).suffix.lstrip(".").lower()
            requires_ocr = file_type in image_types or document_type in image_types
        if requires_ocr != expected:
            bad.append(str(path))
    except (OSError, UnicodeError, json.JSONDecodeError, AttributeError) as error:
        bad.append(f"{path} ({error})")

if not fixture_paths:
    raise SystemExit(f"cohort contains no fixture JSON files: {root}")
if bad:
    label = "OCR-required" if expected else "non-OCR"
    preview = "\n  - ".join(bad[:10])
    raise SystemExit(f"cohort must contain only {label} fixtures; mismatches:\n  - {preview}")
PY
}

run_single() {
  local cohort="$1"
  local output="$2"
  local ocr_flag="$3"
  echo "[bench:local] === single-file run ($ocr_flag): $cohort ==="
  mkdir -p "$output"
  local ocr_args=()
  [ "$ocr_flag" = "OCR enabled" ] && ocr_args=(--ocr)
  "$HARNESS" run \
    --fixtures "$cohort" \
    --frameworks "$FRAMEWORKS" \
    --output "$output" \
    --mode single-file \
    --max-concurrent 1 \
    --iterations "$ITERATIONS" \
    --timeout "$TIMEOUT" \
    --measure-quality \
    --output-format markdown \
    "${ocr_args[@]}" \
    "${SHARD_ARGS[@]}"
  "$HARNESS" gap-report --results "$output" --output "$output"
}

run_batch() {
  local cohort="$1"
  local output="$2"
  local ocr_flag="$3"
  echo "[bench:local] === batch run ($ocr_flag): $cohort ==="
  mkdir -p "$output"
  local ocr_args=()
  [ "$ocr_flag" = "OCR enabled" ] && ocr_args=(--ocr)
  if [ -z "$BATCH_FRAMEWORKS" ]; then
    echo "[bench:local] no verified native-batch framework selected." >&2
    exit 1
  fi
  "$HARNESS" run \
    --fixtures "$cohort" \
    --frameworks "$BATCH_FRAMEWORKS" \
    --output "$output" \
    --mode batch \
    --max-concurrent "$BATCH_WORKERS" \
    --iterations "$ITERATIONS" \
    --timeout "$TIMEOUT" \
    --measure-quality \
    --output-format markdown \
    "${ocr_args[@]}" \
    "${SHARD_ARGS[@]}"
}

if [ -n "$HEURISTIC_FIXTURES" ]; then
  validate_ocr_cohort "$HEURISTIC_FIXTURES" false
  run_single "$HEURISTIC_FIXTURES" "$OUT/single-heuristic" "OCR disabled"
else
  echo "[bench:local] Skipping OCR-disabled single-file cohort: set HEURISTIC_FIXTURES."
fi
if [ -n "$OCR_FIXTURES" ]; then
  validate_ocr_cohort "$OCR_FIXTURES" true
  run_single "$OCR_FIXTURES" "$OUT/single-ocr" "OCR enabled"
else
  echo "[bench:local] Skipping OCR-enabled single-file cohort: set OCR_FIXTURES explicitly."
fi

if [ -n "$BATCH_HEURISTIC_FIXTURES" ]; then
  validate_ocr_cohort "$BATCH_HEURISTIC_FIXTURES" false
  run_batch "$BATCH_HEURISTIC_FIXTURES" "$OUT/batch-heuristic" "OCR disabled"
fi
if [ -n "$BATCH_OCR_FIXTURES" ]; then
  validate_ocr_cohort "$BATCH_OCR_FIXTURES" true
  run_batch "$BATCH_OCR_FIXTURES" "$OUT/batch-ocr" "OCR enabled"
fi
if [ -z "$BATCH_HEURISTIC_FIXTURES" ] && [ -z "$BATCH_OCR_FIXTURES" ]; then
  echo "[bench:local] Skipping batch throughput: set BATCH_HEURISTIC_FIXTURES and/or BATCH_OCR_FIXTURES."
fi

echo ""
echo "[bench:local] Done."
echo "[bench:local]   results root: $OUT"
