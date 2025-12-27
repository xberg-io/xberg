#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Kreuzberg CLI with all features..."
echo "Workspace: $WORKSPACE_ROOT"

cd "$WORKSPACE_ROOT"

cargo build -p kreuzberg-cli --features all

echo ""
echo "Build complete! The CLI binary is now available with all features."
echo "You can now run the CLI server tests successfully."
