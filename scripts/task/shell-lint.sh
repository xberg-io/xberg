#!/usr/bin/env bash
set -euo pipefail

mode="${1:-fix}"
case "$mode" in
fix) format_flag=-w ;;
check) format_flag=-d ;;
*)
  echo "Usage: $0 [fix|check]" >&2
  exit 2
  ;;
esac

files="$(git ls-files '*.sh')"
if [ -z "$files" ]; then
  echo "no tracked shell scripts found; refusing an empty quality check" >&2
  exit 1
fi

scripts=()
while IFS= read -r file; do
  scripts+=("$file")
done <<<"$files"

shfmt "$format_flag" "${scripts[@]}"
shellcheck "${scripts[@]}"
