#!/usr/bin/env bash

set -euo pipefail

RUN_ID="${1:-}"
OUTPUT_DIR="${OUTPUT_DIR:-profiling-results}"

if [ -z "$RUN_ID" ]; then
	echo "Usage: scripts/ci/benchmarks/download-profiling-artifacts.sh <run-id>"
	echo "Example: scripts/ci/benchmarks/download-profiling-artifacts.sh 20406617123"
	exit 1
fi

mkdir -p "$OUTPUT_DIR"

echo "📥 Downloading artifacts from run $RUN_ID..."

gh run download "$RUN_ID" \
	--pattern "profiling-results-*" \
	--dir "$OUTPUT_DIR" 2>/dev/null || echo "No profiling-results artifacts"

gh run download "$RUN_ID" \
	--pattern "flamegraphs-*" \
	--dir "$OUTPUT_DIR" 2>/dev/null || echo "No flamegraphs artifacts"

echo ""
echo "✅ Artifacts downloaded to: $OUTPUT_DIR"
echo ""
echo "📊 Artifact structure:"
tree "$OUTPUT_DIR" -L 2 2>/dev/null || find "$OUTPUT_DIR" -type f | head -20

echo ""
echo "🔍 Validation:"
EMPTY_SVGS=$(find "$OUTPUT_DIR" -name "*.svg" -size 0 2>/dev/null | wc -l | tr -d ' ')
if [ "$EMPTY_SVGS" -gt 0 ]; then
	echo "⚠️  Warning: Found $EMPTY_SVGS empty SVG files"
else
	echo "✅ All SVG flamegraphs are non-empty"
fi

RESULT_FILES=$(find "$OUTPUT_DIR" -name "results.json" 2>/dev/null | wc -l | tr -d ' ')
echo "✅ Found $RESULT_FILES results.json files"

if [ "$RESULT_FILES" -gt 0 ]; then
	FIRST_RESULT=$(find "$OUTPUT_DIR" -name "results.json" | head -1)
	echo ""
	echo "📈 Sample metrics from $(basename "$(dirname "$FIRST_RESULT")"):"
	jq -r '.[] | "\(.framework) - \(.mode): \(.median_time_ms)ms (success: \(.success))"' "$FIRST_RESULT" 2>/dev/null | head -10
fi
