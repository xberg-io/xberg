#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1
setup_rust_ffi_paths "$REPO_ROOT"

echo "=== Compiling Ruby native extension (Verbose Debug) ==="
cd "$REPO_ROOT/packages/ruby"

export CARGO_BUILD_JOBS=1
export RUST_BACKTRACE=1
export RB_SYS_VERBOSE=1

echo ""
echo "=== Pre-compilation environment ==="
echo "Ruby version: $(ruby --version)"
echo "Ruby platform: $(ruby -e 'puts RUBY_PLATFORM')"
echo "Rustc version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo "Working directory: $(pwd)"
echo ""

echo "=== Build configuration variables ==="
echo "CARGO_BUILD_JOBS: ${CARGO_BUILD_JOBS}"
echo "RUST_BACKTRACE: ${RUST_BACKTRACE}"
echo "RB_SYS_VERBOSE: ${RB_SYS_VERBOSE}"
echo "LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
echo ""

echo "=== Pre-vendor directory state ==="
echo "packages/ruby directory contents:"
find . -maxdepth 1 -type f -o -maxdepth 1 -type d | head -20
echo ""

echo "=== Vendoring kreuzberg core ==="
python3 "$REPO_ROOT/scripts/ci/ruby/vendor-kreuzberg-core.py"

echo ""
echo "=== Post-vendor directory state ==="
if [ -d "ext/kreuzberg_rb/vendor" ]; then
	echo "Vendor directory contents:"
	find ext/kreuzberg_rb/vendor -maxdepth 2 -type f | head -10
else
	echo "WARNING: No vendor directory found in ext/kreuzberg_rb"
fi
echo ""

echo "=== Running rake compile with verbose output ==="
bundle exec rake compile --verbose --trace 2>&1 || {
	echo ""
	echo "ERROR: rake compile failed"
	echo "=== Attempting to capture compilation error details ==="

	if [ -f "mkmf.log" ]; then
		echo "=== mkmf.log (last 150 lines) ==="
		tail -150 mkmf.log
	fi

	echo ""
	echo "=== Looking for compiled artifacts ==="
	find . -name "*.so" -o -name "*.dll" -o -name "*.dylib" 2>/dev/null | head -20

	echo ""
	echo "=== Checking gem installation ==="
	gem list kreuzberg || echo "Gem not found"

	exit 1
}

echo ""
echo "=== Post-compilation directory state ==="
echo "lib/ contents:"
if [ -d "lib" ]; then
	find lib -type f -name "*.so" -o -name "*.dll" -o -name "*.dylib" 2>/dev/null || echo "No compiled extension found"
else
	echo "ERROR: lib directory not found"
fi
echo ""

echo "=== Verifying extension can be loaded ==="
ruby -e "require_relative 'lib/kreuzberg'; puts 'Extension loaded successfully'" 2>&1 || {
	echo "WARNING: Could not load extension directly"
	echo "This might be expected if gem installation is required"
}

echo ""
echo "=== Compilation complete ==="
