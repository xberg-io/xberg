#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

echo "=========================================="
echo "Setting library paths for test execution"
echo "=========================================="

setup_go_paths "$REPO_ROOT"

if [[ "${RUNNER_OS:-}" == "Windows" ]]; then
	{
		echo "PATH=${PATH}"
		echo "CGO_ENABLED=${CGO_ENABLED:-}"
		echo "CGO_CFLAGS=${CGO_CFLAGS:-}"
		echo "PKG_CONFIG_PATH=${PKG_CONFIG_PATH:-}"
	} >>"$GITHUB_ENV"
else
	{
		echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH:-}"
		echo "DYLD_LIBRARY_PATH=${DYLD_LIBRARY_PATH:-}"
		echo "DYLD_FALLBACK_LIBRARY_PATH=${DYLD_FALLBACK_LIBRARY_PATH:-}"
		echo "PATH=${PATH}"
		echo "CGO_ENABLED=${CGO_ENABLED:-}"
		echo "CGO_CFLAGS=${CGO_CFLAGS:-}"
		echo "CGO_LDFLAGS=${CGO_LDFLAGS:-}"
		echo "PKG_CONFIG_PATH=${PKG_CONFIG_PATH:-}"
	} >>"$GITHUB_ENV"
fi

echo "✓ Library paths set successfully"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
