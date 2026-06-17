#!/usr/bin/env bash

set -euo pipefail

{
  cc_bin="aarch64-linux-gnu-gcc"
  cxx_bin="aarch64-linux-gnu-g++"
  ar_bin="aarch64-linux-gnu-ar"
  if ! command -v "$cc_bin" >/dev/null 2>&1; then
    cc_bin="gcc"
  fi
  if ! command -v "$cxx_bin" >/dev/null 2>&1; then
    cxx_bin="g++"
  fi
  if ! command -v "$ar_bin" >/dev/null 2>&1; then
    ar_bin="ar"
  fi

  echo "CC_aarch64_unknown_linux_gnu=${cc_bin}"
  echo "CXX_aarch64_unknown_linux_gnu=${cxx_bin}"
  echo "AR_aarch64_unknown_linux_gnu=${ar_bin}"
  echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=${cc_bin}"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"
