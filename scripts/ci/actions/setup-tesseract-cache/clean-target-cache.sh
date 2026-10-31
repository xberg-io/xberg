#!/usr/bin/env bash
set -euo pipefail

rust_target="${1:?rust target required}"
case "$rust_target" in
. | .. | *[!A-Za-z0-9._-]*)
  echo "invalid rust target: $rust_target" >&2
  exit 2
  ;;
esac

rm -rf "target/${rust_target}/xberg-tesseract-cache"
