#!/usr/bin/env bash

set -euo pipefail

target="${CLI_TARGET:-}"

case "$target" in
x86_64-unknown-linux-musl)
  {
    echo "CC_x86_64_unknown_linux_musl=musl-gcc"
    echo "CXX_x86_64_unknown_linux_musl=clang++"
    echo "AR_x86_64_unknown_linux_musl=ar"
    echo "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc"
  } >>"${GITHUB_ENV:?GITHUB_ENV not set}"
  ;;
aarch64-unknown-linux-musl)
  {
    echo "CC_aarch64_unknown_linux_musl=musl-gcc"
    echo "CXX_aarch64_unknown_linux_musl=clang++"
    echo "AR_aarch64_unknown_linux_musl=ar"
    echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc"
  } >>"${GITHUB_ENV:?GITHUB_ENV not set}"
  ;;
*)
  echo "configure-musl-linker: no configuration needed for target $target" >&2
  ;;
esac
