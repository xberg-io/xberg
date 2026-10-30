# shellcheck shell=bash
# Install a cargo-provided binary at an exact version unless that version is already on PATH;
# `<binary> --version` prints "<name> <version>". ~keep
cargo_install_pinned() {
  local binary="$1" crate="$2" version="$3"
  if command -v "$binary" >/dev/null 2>&1 &&
    [ "$("$binary" --version 2>/dev/null | awk '{print $2}')" = "$version" ]; then
    echo "$crate $version already installed"
    return
  fi
  cargo install "$crate" --version "$version" --locked
}
