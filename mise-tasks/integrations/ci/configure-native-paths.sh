#!/usr/bin/env bash
#MISE description="Export the native library search paths the integration jobs need to GITHUB_ENV"
set -euo pipefail

: "${GITHUB_ENV:?GITHUB_ENV must name the file later workflow steps read their environment from}"
export PKG_CONFIG_PATH="${PWD}/crates/xberg-ffi:${PKG_CONFIG_PATH:-}"
export LD_LIBRARY_PATH="/usr/local/lib:${PWD}/target/release:${ORT_LIB_LOCATION:-}:${LD_LIBRARY_PATH:-}"
echo "PKG_CONFIG_PATH=${PKG_CONFIG_PATH}" >>"$GITHUB_ENV"
echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}" >>"$GITHUB_ENV"
