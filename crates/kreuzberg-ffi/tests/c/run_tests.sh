#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REPO_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

echo "==> Building kreuzberg-ffi..."
cargo build -p kreuzberg-ffi --manifest-path "$REPO_DIR/Cargo.toml"

echo "==> Compiling C tests..."
make -C "$SCRIPT_DIR" LIBDIR="$REPO_DIR/target/debug" clean all

echo "==> Running C tests..."
FAILED=0
for test in "$SCRIPT_DIR"/test_*; do
  [ -x "$test" ] || continue
  name=$(basename "$test")
  if "$test"; then
    echo "  PASS: $name"
  else
    echo "  FAIL: $name"
    FAILED=1
  fi
done

if [ "$FAILED" -eq 0 ]; then
  echo "==> All C tests passed!"
else
  echo "==> Some C tests FAILED"
  exit 1
fi
