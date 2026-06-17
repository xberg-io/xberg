#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

setup_go_paths "$REPO_ROOT"

echo "Go bindings build environment:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
echo "  CGO_ENABLED: ${CGO_ENABLED:-<not set>}"
echo

cd "$REPO_ROOT/packages/go/v4"
echo "Building Go bindings in: $(pwd)"
go build -v ./...

echo "Go bindings build complete"
