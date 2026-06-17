#!/usr/bin/env bash
set -euo pipefail

if command -v wasm-pack >/dev/null 2>&1; then
  wasm-pack --version
  exit 0
fi

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
  cargo install wasm-pack --locked --force
  ;;
*)
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
  ;;
esac

# Ensure ~/.cargo/bin is in PATH for subsequent steps (wasm-pack installs here)
if [[ -n "${GITHUB_PATH:-}" && -d "$HOME/.cargo/bin" ]]; then
  echo "$HOME/.cargo/bin" >>"$GITHUB_PATH"
fi
