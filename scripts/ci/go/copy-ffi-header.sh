#!/usr/bin/env bash
# Copy FFI header from kreuzberg-ffi crate to Go bindings
# Used by: ci-go.yaml - after building FFI library
# Ensures Go code can find the FFI header at internal/ffi/kreuzberg.h

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

SOURCE_HEADER="${REPO_ROOT}/crates/kreuzberg-ffi/kreuzberg.h"
DEST_DIR="${REPO_ROOT}/packages/go/v4/internal/ffi"
DEST_HEADER="${DEST_DIR}/kreuzberg.h"

echo "=========================================="
echo "Copying FFI Header for Go Bindings"
echo "=========================================="

if [[ ! -f "$SOURCE_HEADER" ]]; then
	echo "ERROR: Source header not found at $SOURCE_HEADER"
	exit 1
fi

echo "Source: $SOURCE_HEADER"
echo "Destination: $DEST_HEADER"
echo ""

mkdir -p "$DEST_DIR"
cp "$SOURCE_HEADER" "$DEST_HEADER"

echo "✓ FFI header copied successfully"
echo "Verifying header..."
if [[ -f "$DEST_HEADER" ]]; then
	echo "✓ Header exists at destination"
	LINES=$(wc -l <"$DEST_HEADER")
	echo "  Header size: $LINES lines"
else
	echo "ERROR: Header not found at destination after copy"
	exit 1
fi

echo ""
echo "=========================================="
echo "FFI Header Copy Completed"
echo "=========================================="
