#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

"${REPO_ROOT}/scripts/download_pdfium_runtime.sh"

setup_go_paths "$REPO_ROOT"

cd "${REPO_ROOT}/packages/go/v4"
go test ./...
