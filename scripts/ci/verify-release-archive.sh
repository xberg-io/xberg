#!/usr/bin/env bash
# Verify that a release step produced exactly one archive and that it carries no
# GPL-2.0 x265. The empty-glob guard is the point: a mistyped path must not make
# the x265 gate pass by scanning nothing.
#
# Usage: verify-release-archive.sh '<archive-glob>'
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 '<archive-glob>'" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pattern="$1"
gate="$SCRIPT_DIR/verify_no_gpl_codecs.py"
if [ ! -f "$gate" ]; then
  echo "::error::missing $gate beside $0; both must be staged from the same workflow revision" >&2
  exit 1
fi
shopt -s nullglob
# The argument is a glob by design, so it must stay unquoted here. ~keep
# shellcheck disable=SC2206
archives=($pattern)
if [ "${#archives[@]}" -ne 1 ]; then
  echo "::error::expected exactly one release archive matching '$pattern', found ${#archives[@]}" >&2
  exit 1
fi

if [ "${RUNNER_OS:-}" = "Windows" ] || ! command -v python3 >/dev/null 2>&1; then
  python "$SCRIPT_DIR/verify_no_gpl_codecs.py" --min-artifacts 1 "${archives[0]}"
else
  python3 "$SCRIPT_DIR/verify_no_gpl_codecs.py" --min-artifacts 1 "${archives[0]}"
fi
