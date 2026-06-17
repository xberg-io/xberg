#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

LIB_DIR="$REPO_ROOT/target/release"

if [ ! -d "$LIB_DIR" ]; then
  echo "::error::Native library directory not found at $LIB_DIR" >&2
  exit 1
fi

setup_all_library_paths "$REPO_ROOT"

echo "Elixir bindings build environment:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LIB_DIR: $LIB_DIR"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-}"
echo ""

cd "$REPO_ROOT/packages/elixir"
echo "Building Elixir bindings in: $(pwd)"

echo "Installing Elixir dependencies..."
mix deps.get

echo "Compiling Elixir package..."
MIX_ENV=prod mix compile

echo "Elixir bindings built successfully"
