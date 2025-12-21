#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Check if flamegraphs directory exists
if [ ! -d "flamegraphs" ]; then
	echo "No flamegraphs directory found, skipping index generation"
	exit 0
fi

# Count flamegraphs
FLAMEGRAPH_COUNT=$(find flamegraphs -name "*.svg" 2>/dev/null | wc -l)
echo "Found $FLAMEGRAPH_COUNT flamegraph(s)"

if [ "$FLAMEGRAPH_COUNT" -eq 0 ]; then
	echo "No flamegraphs to index, skipping"
	exit 0
fi

# Create output directory
mkdir -p benchmark-output/flamegraphs

# Copy flamegraphs
echo "Copying flamegraphs to benchmark output..."
cp -r flamegraphs/* benchmark-output/flamegraphs/

# Generate HTML index using benchmark harness
echo "Generating flamegraph index..."
"${REPO_ROOT}/target/release/benchmark-harness" generate-flamegraph-index \
	--flamegraphs benchmark-output/flamegraphs \
	--output benchmark-output/flamegraphs.html

if [ -f "benchmark-output/flamegraphs.html" ]; then
	echo "✓ Flamegraph index generated: benchmark-output/flamegraphs.html"
	ls -lh benchmark-output/flamegraphs.html
else
	echo "✗ Failed to generate flamegraph index"
	exit 1
fi
