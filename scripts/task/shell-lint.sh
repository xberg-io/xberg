#!/usr/bin/env bash
set -euo pipefail

mode="${1:-fix}"

files="$(git ls-files '*.sh' || true)"
if [ -z "$files" ]; then
  exit 0
fi

case "$mode" in
fix)
  # shellcheck disable=SC2086
  shfmt -w $files
  ;;
check)
  # shellcheck disable=SC2086
  shfmt -d $files
  ;;
*)
  echo "Usage: $0 [fix|check]" >&2
  exit 2
  ;;
esac

# shellcheck disable=SC2086
shellcheck $files
