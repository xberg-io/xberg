#!/usr/bin/env bash
#MISE description="Report the newest bblanchon/pdfium-binaries build against the pin, with candidate checksums (edits nothing)"
#USAGE flag "--write-checksums <file>" help="Write the candidate checksum manifest (release API digests) to this file"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
args=()
[ -n "${usage_write_checksums:-}" ] && args+=(--write-checksums "$usage_write_checksums")
exec bash "$root/scripts/pdfium/pdfium.sh" latest ${args[@]+"${args[@]}"}
