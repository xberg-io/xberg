#!/usr/bin/env bash
#MISE description="Provision if needed, then print export lines (PDFIUM_DYNAMIC_LIB_PATH, library paths, XBERG_REQUIRE_PDFIUM) for the provisioned libpdfium"
#USAGE flag "--version <build>" help="Chromium build (default: scripts/pdfium-version)"
#USAGE flag "--platform <id>" help="Platform id (default: this host)"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
args=()
[ -n "${usage_version:-}" ] && args+=(--version "$usage_version")
[ -n "${usage_platform:-}" ] && args+=(--platform "$usage_platform")
exec bash "$root/scripts/pdfium/pdfium.sh" env ${args[@]+"${args[@]}"}
