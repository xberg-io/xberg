#!/usr/bin/env bash
#MISE description="Download the pinned, checksum-verified libpdfium into the cache and print its directory"
#USAGE flag "--version <build>" help="Chromium build to provision (default: scripts/pdfium-version)"
#USAGE flag "--platform <id>" help="linux-x64, linux-arm64, linux-musl-x64, linux-musl-arm64, mac-x64, mac-arm64, win-x64 or win-arm64 (default: this host)"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
args=()
hint=""
[ -n "${usage_version:-}" ] && args+=(--version "$usage_version") && hint+=" --version $(printf '%q' "$usage_version")"
[ -n "${usage_platform:-}" ] && args+=(--platform "$usage_platform") && hint+=" --platform $(printf '%q' "$usage_platform")"
dir="$(bash "$root/scripts/pdfium/pdfium.sh" provision ${args[@]+"${args[@]}"})"
echo "$dir"
echo "Use it: eval \"\$(mise run -q pdfium:env${hint})\"   (exports PDFIUM_DYNAMIC_LIB_PATH=$dir)" >&2
