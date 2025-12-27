#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

setup_rust_ffi_paths "$REPO_ROOT"

cd "$REPO_ROOT"

if ! command -v wasm-pack >/dev/null 2>&1; then
	cargo install wasm-pack --locked
fi

rustup target add wasm32-unknown-unknown

saved_rustflags="${RUSTFLAGS:-}"
unset RUSTFLAGS

pnpm install
pnpm -C crates/kreuzberg-wasm run build

if [ -n "$saved_rustflags" ]; then
	export RUSTFLAGS="$saved_rustflags"
fi
