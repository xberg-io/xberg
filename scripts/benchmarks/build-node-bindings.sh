#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

TARGET="${TARGET:-}"

if [ -z "$TARGET" ]; then
	echo "::error::TARGET environment variable is required" >&2
	exit 1
fi

setup_all_library_paths "$REPO_ROOT"

cd "$REPO_ROOT"
pnpm install

cd "$REPO_ROOT/crates/kreuzberg-node"
pnpm exec napi build --platform --release --target "${TARGET}"
pnpm run build:ts
pkg=$(pnpm pack | tail -n1 | tr -d '\r')
cd "$REPO_ROOT"
pnpm add --workspace-root "file:crates/kreuzberg-node/${pkg}"
