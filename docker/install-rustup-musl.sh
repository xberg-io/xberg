#!/bin/sh
set -eu

toolchain=${1:?Rust toolchain required}
version=1.29.3
arch=$(uname -m)
# Match the official archive's rustup-init.sha256 for each musl host.
case "$arch" in
x86_64) digest=1552169aba2836777136ccff7a381e95c17233b5d8250cd5e10ae3cb971bfcfc ;;
aarch64) digest=956deacdc27ab82ae7ac821d5d3e9e95e4795d476d17efe99743bb8531ca1031 ;;
*)
  printf 'Unsupported Rustup architecture: %s\n' "$arch" >&2
  exit 1
  ;;
esac

scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
cd "$scratch"
curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
  "https://static.rust-lang.org/rustup/archive/$version/$arch-unknown-linux-musl/rustup-init" \
  -o rustup-init
printf '%s  rustup-init\n' "$digest" >rustup-init.sha256
sha256sum -c rustup-init.sha256
chmod 0755 rustup-init
./rustup-init -y --default-toolchain "$toolchain" --component rust-src
