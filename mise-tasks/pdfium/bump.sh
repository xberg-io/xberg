#!/usr/bin/env bash
#MISE description="Rewrite the pdfium pin and checksums from a release's published digests (dry run unless --apply)"
#USAGE arg "<build>" help="Chromium build number, e.g. 8165"
#USAGE flag "--apply" help="Write scripts/pdfium-version and the checksums file"
#USAGE flag "--allow-binding-mismatch" help="Allow a build with no checked-in bindgen/pdfium_<build>.rs"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
args=()
[ "${usage_apply:-false}" = true ] && args+=(--apply)
[ "${usage_allow_binding_mismatch:-false}" = true ] && args+=(--allow-binding-mismatch)
exec bash "$root/scripts/pdfium/pdfium.sh" bump "$usage_build" ${args[@]+"${args[@]}"}
