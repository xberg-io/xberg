#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== Building R source package ==="
cd "$REPO_ROOT/packages/r"

# Vendor core crate
python3 "$REPO_ROOT/scripts/ci/r/vendor-kreuzberg-core.py"

# Build source package
R CMD build .

echo "=== R source package built ==="
ls -la kreuzberg_*.tar.gz
