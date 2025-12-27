#!/usr/bin/env bash
set -euo pipefail

echo "Cleaning up large build artifacts to reduce cache size..."

rm -rf target/*/build/kreuzberg-*/out/pdfium 2>/dev/null || true

find target -type f -name "*.rlib" -size +10M -delete 2>/dev/null || true
find target -type f -name "*.so" -size +10M -delete 2>/dev/null || true
find target -type f -name "*.dylib" -size +10M -delete 2>/dev/null || true
find target -type f -name "*.dll" -size +10M -delete 2>/dev/null || true

rm -rf target/*/incremental 2>/dev/null || true

echo "Cleanup completed successfully"

du -sh target 2>/dev/null || echo "No target directory found"
