#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-${TARGET:-}}"
if [[ -z "$TARGET" ]]; then
  echo "Usage: $0 <target>" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="$ROOT/xberg-cli-${TARGET}"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$ROOT/target/${TARGET}/release/xberg" "$STAGE/"
python3 "$ROOT/scripts/ci/package-notices.py" stage "$STAGE"
cp "$ROOT/README.md" "$STAGE/"

if [[ -f "$ROOT/target/${TARGET}/release/libpdfium.dylib" ]]; then
  cp "$ROOT/target/${TARGET}/release/libpdfium.dylib" "$STAGE/"
fi
if [[ -f "$ROOT/target/${TARGET}/release/libpdfium.so" ]]; then
  cp "$ROOT/target/${TARGET}/release/libpdfium.so" "$STAGE/"
fi

tar -czf "xberg-cli-${TARGET}.tar.gz" -C "$ROOT" "xberg-cli-${TARGET}"
python3 "$ROOT/scripts/ci/package-notices.py" verify "xberg-cli-${TARGET}.tar.gz" "xberg-cli-${TARGET}"
rm -rf "$STAGE"
