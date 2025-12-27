#!/usr/bin/env bash

set -euo pipefail

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
	echo "Usage: package-cli-unix.sh <target>"
	echo "  target: Rust build target"
	exit 1
fi

echo "=== Packaging CLI binary for $TARGET ==="

cd "target/$TARGET/release"
tar czf "kreuzberg-cli-$TARGET.tar.gz" kreuzberg
mv "kreuzberg-cli-$TARGET.tar.gz" ../../..

echo "Packaging complete: kreuzberg-cli-$TARGET.tar.gz"
