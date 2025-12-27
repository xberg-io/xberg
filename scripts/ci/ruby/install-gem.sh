#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Installing Ruby gem ==="
cd "$REPO_ROOT/packages/ruby"
gem install pkg/kreuzberg-*.gem --no-document
echo "Gem installation complete"
