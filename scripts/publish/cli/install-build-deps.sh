#!/usr/bin/env bash

set -euo pipefail

target="${CLI_TARGET:-}"

sudo apt-get update
case "$target" in
aarch64-unknown-linux-gnu)
  sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
  ;;
x86_64-unknown-linux-musl | aarch64-unknown-linux-musl)
  sudo apt-get install -y musl-tools clang libc++-dev libc++abi-dev
  ;;
*) ;;
esac
