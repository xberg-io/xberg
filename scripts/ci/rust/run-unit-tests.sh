#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT"

echo "=== Running Rust unit tests ==="

setup_tessdata

echo "Test environment configuration:"
echo "  TESSDATA_PREFIX: ${TESSDATA_PREFIX:-not set}"
echo "  RUST_BACKTRACE: ${RUST_BACKTRACE:-not set}"
echo "  CARGO_TERM_COLOR: ${CARGO_TERM_COLOR:-not set}"

echo "Workspace information:"
echo "  Repository: $REPO_ROOT"
echo "  Excluded packages: xberg-e2e-generator, xberg-py, xberg-node, xberg-candle-ocr, xberg-cli, benchmark-harness"

if [ ! -d "$TESSDATA_PREFIX" ]; then
  echo "WARNING: TESSDATA_PREFIX directory not found: $TESSDATA_PREFIX"
  echo "Attempting to create it..."
  mkdir -p "$TESSDATA_PREFIX"
  ensure_tessdata "$TESSDATA_PREFIX"
fi

echo "Verifying Tesseract data files..."
for lang in eng osd; do
  langfile="$TESSDATA_PREFIX/${lang}.traineddata"
  if [ -f "$langfile" ]; then
    size=$(stat -f%z "$langfile" 2>/dev/null || stat -c%s "$langfile" 2>/dev/null || echo "unknown")
    echo "  ✓ ${lang}.traineddata (${size} bytes)"
  else
    echo "  WARNING: Missing ${lang}.traineddata"
  fi
done

if [ -n "${XBERG_PDFIUM_PREBUILT:-}" ]; then
  export LD_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${LD_LIBRARY_PATH:-}"
  export DYLD_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${DYLD_LIBRARY_PATH:-}"
  export DYLD_FALLBACK_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${DYLD_FALLBACK_LIBRARY_PATH:-}"
  echo "Library path configuration:"
  echo "  LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
  echo "  DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
  echo "  DYLD_FALLBACK_LIBRARY_PATH: $DYLD_FALLBACK_LIBRARY_PATH"
fi

echo "=== Starting cargo test ==="

# NOTE: We intentionally avoid `--all-features` for the `xberg` crate because
TEST_LOG="/tmp/cargo-test-$$.log"

if ! {
  # ~keep `--all-targets` runs --lib --bins --tests --examples --benches but excludes
  # ~keep `--doc`. The xberg crate still has rustdoc examples for private/internal
  # ~keep APIs; `cargo test -p xberg --features full --doc` currently fails those
  # ~keep examples because rustdoc compiles them as an external crate.
  echo "=== cargo test -p xberg --features full ==="
  RUST_BACKTRACE=full cargo test --locked -p xberg --features full --all-targets --verbose

  echo "=== cargo test --workspace (all features, excluding xberg) ==="
  extra_excludes=()
  extra_excludes+=(--exclude xberg-candle-ocr)
  extra_excludes+=(--exclude xberg-cli)
  extra_excludes+=(--exclude benchmark-harness)
  RUST_BACKTRACE=full cargo test --locked \
    --workspace \
    --exclude xberg \
    --exclude xberg-e2e-generator \
    --exclude xberg-py \
    --exclude xberg-node \
    ${extra_excludes[@]+"${extra_excludes[@]}"} \
    --all-features \
    --all-targets \
    --verbose
} 2>&1 | tee "$TEST_LOG"; then
  echo "=== Test execution failed ==="
  echo "Last 50 lines of test output:"
  tail -n 50 "$TEST_LOG"
  echo ""
  echo "Collecting diagnostic information..."
  echo "Disk space:"
  df -h . || du -h . 2>/dev/null | head -1
  echo "Cargo environment:"
  cargo --version
  rustc --version
  rm -f "$TEST_LOG"
  exit 1
fi

rm -f "$TEST_LOG"

echo "=== Tests complete ==="
