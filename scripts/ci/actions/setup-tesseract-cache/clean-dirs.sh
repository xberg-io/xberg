#!/usr/bin/env bash
set -euo pipefail

label="${1:?label required}"

case "$label" in
. | .. | *[!A-Za-z0-9._-]*)
  echo "invalid cache label: $label" >&2
  exit 2
  ;;
esac

rm -rf ".tesseract-cache/${label}"
rm -rf ".xdg-cache/${label}"
