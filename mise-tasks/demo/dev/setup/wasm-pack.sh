#!/usr/bin/env bash
#MISE description="Install wasm-pack 0.13.1 (matches CI pinned version)"
set -euo pipefail

# shellcheck source=mise-tasks/.lib/cargo-install-pinned.sh
. "$MISE_CONFIG_ROOT/mise-tasks/.lib/cargo-install-pinned.sh"
cargo_install_pinned wasm-pack wasm-pack 0.13.1
