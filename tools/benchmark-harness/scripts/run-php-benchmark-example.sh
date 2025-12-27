#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BENCHMARK_BIN="$REPO_ROOT/target/release/benchmark-harness"
FIXTURES_DIR="$REPO_ROOT/tools/benchmark-harness/fixtures"
OUTPUT_DIR="${OUTPUT_DIR:-$REPO_ROOT/results/php-benchmarks}"

echo "=== Kreuzberg PHP Benchmark Runner ==="
echo "Repository: $REPO_ROOT"
echo "Benchmark binary: $BENCHMARK_BIN"
echo "Fixtures: $FIXTURES_DIR"
echo "Output: $OUTPUT_DIR"
echo ""

echo "Checking prerequisites..."

if ! command -v php &>/dev/null; then
	echo "ERROR: PHP not found. Please install PHP 8.2 or higher."
	exit 1
fi

PHP_VERSION=$(php -r 'echo PHP_VERSION;')
echo "✓ PHP $PHP_VERSION found"

if [ ! -f "$BENCHMARK_BIN" ]; then
	echo "ERROR: Benchmark binary not found at $BENCHMARK_BIN"
	echo "Please build it with: cargo build --release -p benchmark-harness"
	exit 1
fi
echo "✓ Benchmark harness binary found"

if [ ! -d "$FIXTURES_DIR" ]; then
	echo "ERROR: Fixtures directory not found at $FIXTURES_DIR"
	exit 1
fi
echo "✓ Fixtures directory found"

if [ ! -d "$REPO_ROOT/packages/php/vendor" ]; then
	echo "ERROR: PHP composer dependencies not installed"
	echo "Please run: cd packages/php && composer install"
	exit 1
fi
echo "✓ Composer dependencies installed"

if ! php -r 'if (!function_exists("kreuzberg_extract_file")) exit(1);' 2>/dev/null; then
	echo "WARNING: Kreuzberg PHP extension not loaded"
	echo "The benchmarks will fail until the extension is built and loaded."
	echo ""
	echo "To build the extension:"
	echo "  cd crates/kreuzberg-php"
	echo "  bash build.sh"
	echo ""
	echo "To load the extension, add to php.ini or use -d flag:"
	echo "  php -d extension=/path/to/kreuzberg.so"
	echo ""
	read -p "Continue anyway? (y/N) " -n 1 -r
	echo
	if [[ ! $REPLY =~ ^[Yy]$ ]]; then
		exit 1
	fi
else
	echo "✓ Kreuzberg PHP extension loaded"
fi

echo ""
echo "=== Running PHP Benchmarks ==="
echo ""

mkdir -p "$OUTPUT_DIR"

echo "Example 1: PHP sync adapter on small PDFs"
"$BENCHMARK_BIN" run \
	--fixtures "$FIXTURES_DIR/pdf_small.json" \
	--frameworks kreuzberg-php-sync \
	--output "$OUTPUT_DIR/sync-small-pdf" \
	--mode single-file \
	--iterations 3 \
	--format both

echo ""
echo "Results: $OUTPUT_DIR/sync-small-pdf/"
echo ""

echo "Example 2: PHP batch adapter on multiple document types"
"$BENCHMARK_BIN" run \
	--fixtures "$FIXTURES_DIR" \
	--frameworks kreuzberg-php-batch \
	--output "$OUTPUT_DIR/batch-all-types" \
	--mode batch \
	--iterations 3 \
	--format both

echo ""
echo "Results: $OUTPUT_DIR/batch-all-types/"
echo ""

echo "Example 3: Compare PHP with Python, Ruby, and Node"
"$BENCHMARK_BIN" run \
	--fixtures "$FIXTURES_DIR/pdf_medium.json" \
	--frameworks kreuzberg-php-sync,kreuzberg-python-sync,kreuzberg-ruby-sync,kreuzberg-node-async \
	--output "$OUTPUT_DIR/language-comparison" \
	--mode single-file \
	--iterations 5 \
	--format both

echo ""
echo "Results: $OUTPUT_DIR/language-comparison/"
echo "View HTML report: open $OUTPUT_DIR/language-comparison/index.html"
echo ""

echo "=== Benchmarks Complete ==="
echo ""
echo "All results saved to: $OUTPUT_DIR"
echo ""
echo "To view results:"
echo "  - JSON: cat $OUTPUT_DIR/*/results.json"
echo "  - HTML: open $OUTPUT_DIR/*/index.html"
echo ""
