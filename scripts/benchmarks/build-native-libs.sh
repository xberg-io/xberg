#!/usr/bin/env bash
# Builds workspace with all features and benchmark harness in release mode
# No required environment variables

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

# Setup all library paths (PDFium + ONNX + Rust FFI)
setup_all_library_paths "$REPO_ROOT"

echo "Building native libraries in release mode:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
echo

cd "$REPO_ROOT"
# Build with all features except mutually-exclusive PDF linking strategies
# The default PDF strategy (dynamic linking) is used when 'pdf' feature is enabled
# without pdf-static, pdf-bundled, or pdf-system
cargo build --workspace --release \
	--features full,profiling,api,mcp,otel
cargo build --manifest-path tools/benchmark-harness/Cargo.toml --release --features profiling

# Copy static libraries to target/release for downstream jobs (e.g., Ruby native extension)
# that need to link against libkreuzberg_ffi.a
if [ -d "$REPO_ROOT/target/release" ]; then
	find "$REPO_ROOT/target" -type f -name "*.a" -exec cp -v {} "$REPO_ROOT/target/release/" \; 2>/dev/null || true
fi

echo "Native libraries build complete"
