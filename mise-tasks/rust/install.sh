#!/usr/bin/env bash
#MISE description="Install Rust toolchain and components"
set -euo pipefail

# rust-toolchain.toml pins the toolchain, so rustup installs that rather than tracking stable. ~keep
rustup toolchain install
rustup component add rustfmt clippy
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*) rustup component add llvm-tools ;;
esac
# cargo-edit (`cargo upgrade`, used by the upgrade tasks) publishes no release binaries, so mise
# could only build it through the cargo backend, which locks nothing; it stays a cargo install. ~keep
cargo install cargo-edit --version 0.13.13 --locked
cargo --version
rustc --version
