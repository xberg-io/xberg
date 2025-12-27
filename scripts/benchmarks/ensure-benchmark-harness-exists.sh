#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

source "${REPO_ROOT}/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

if [ ! -d "$REPO_ROOT/tools/benchmark-harness" ]; then
	echo "::error::tools/benchmark-harness not found on branch ${GITHUB_REF}." >&2
	exit 1
fi

echo "✓ Benchmark harness directory verified at: $REPO_ROOT/tools/benchmark-harness"
