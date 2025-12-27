#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
	echo "Usage: test-cli-unix.sh <target>"
	echo "  target: Rust build target"
	exit 1
fi

echo "=== Testing CLI binary for $TARGET ==="

setup_pdfium_paths

tar xzf "kreuzberg-cli-$TARGET.tar.gz"
chmod +x kreuzberg
./kreuzberg --version
./kreuzberg --help

echo "CLI tests passed!"
