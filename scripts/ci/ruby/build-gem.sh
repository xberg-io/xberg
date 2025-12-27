#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Building Ruby gem (Comprehensive Debug) ==="
cd "$REPO_ROOT/packages/ruby"

echo ""
echo "=== Ruby Environment ==="
echo "Ruby version: $(ruby --version)"
echo "Ruby platform: $(ruby -e 'puts RUBY_PLATFORM')"
echo "Bundler version: $(bundle --version)"
echo "Gem version: $(gem --version)"
echo "rb_sys version: $(gem list rb_sys)"
echo "Working directory: $(pwd)"
echo ""

echo "=== Rust Compilation Environment ==="
echo "Rustc version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo "Rustc details:"
rustc -vV | grep -E "(host|release)" || true
echo ""

echo "=== System Information ==="
if [ "$(uname)" = "Linux" ]; then
	echo "OS: Linux"
elif [ "$(uname)" = "Darwin" ]; then
	echo "OS: macOS"
	echo "Arch: $(uname -m)"
elif [ "$(uname)" = "MINGW64_NT" ] || [ "$(uname)" = "MSYS_NT" ]; then
	echo "OS: Windows (MINGW/MSYS)"
else
	echo "OS: $(uname -s)"
fi
echo ""

echo "=== Build Environment Variables ==="
echo "RB_SYS_CARGO_PROFILE: ${RB_SYS_CARGO_PROFILE:-<not set>}"
echo "RB_SYS_VERBOSE: ${RB_SYS_VERBOSE:-<not set>}"
echo "CARGO_BUILD_JOBS: ${CARGO_BUILD_JOBS:-<not set>}"
echo "CARGO_INCREMENTAL: ${CARGO_INCREMENTAL:-<not set>}"
echo "RUST_BACKTRACE: ${RUST_BACKTRACE:-<not set>}"
echo ""

echo "=== Bundler Configuration ==="
bundle config list || true
echo ""

echo "=== Rakefile Information ==="
if [ -f Rakefile ]; then
	echo "Rakefile exists"
	grep -E "ExtensionTask|ext_dir" Rakefile || echo "No ExtensionTask found"
fi
echo ""

echo "=== Pre-compilation Directory State ==="
echo "packages/ruby contents:"
find . -maxdepth 1 -type f -o -maxdepth 1 -type d | head -20
echo ""
if [ -d "ext/kreuzberg_rb" ]; then
	echo "ext/kreuzberg_rb contents:"
	find ext/kreuzberg_rb -maxdepth 1 -type f -o -maxdepth 1 -type d | head -20
else
	echo "ERROR: ext/kreuzberg_rb directory not found"
fi
echo ""

echo "=== Running rake compile (verbose) ==="
bundle exec rake compile --verbose --trace 2>&1 || {
	echo "ERROR: rake compile failed"
	echo "=== Looking for error details ==="
	if [ -f "mkmf.log" ]; then
		echo "mkmf.log contents (last 100 lines):"
		tail -100 mkmf.log
	fi
	exit 1
}
echo ""

echo "=== Post-compile Directory State ==="
echo "lib/ contents:"
if [ -d "lib" ]; then
	find lib -type f | head -20
else
	echo "WARNING: lib directory not found"
fi
echo ""

echo "=== Running rake build (verbose) ==="
bundle exec rake build --verbose --trace 2>&1 || {
	echo "ERROR: rake build failed"
	exit 1
}
echo ""

echo "=== Gem build artifacts ==="
if [ -d "pkg" ]; then
	ls -lh pkg/*.gem || echo "No gem files found in pkg/"
else
	echo "ERROR: pkg directory not found"
fi
echo ""

echo "=== Build Success ==="
echo "Gem build complete successfully"
