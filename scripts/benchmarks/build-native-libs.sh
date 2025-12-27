#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

setup_all_library_paths "$REPO_ROOT"

echo "Building native libraries in release mode:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
echo

cd "$REPO_ROOT"
cargo build --workspace --release \
	--features full,profiling,api,mcp,otel
cargo build --manifest-path tools/benchmark-harness/Cargo.toml --release --features profiling

if [ -d "$REPO_ROOT/target/release" ]; then
	find "$REPO_ROOT/target" -type f -name "*.a" -exec cp -v {} "$REPO_ROOT/target/release/" \; 2>/dev/null || true
fi

echo "Native libraries build complete"
