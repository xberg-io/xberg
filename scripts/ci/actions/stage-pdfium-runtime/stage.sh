#!/usr/bin/env bash
set -euo pipefail

destination="${1:?destination required}"
additional_destinations="${2:-}"

if [ -z "${KREUZBERG_PDFIUM_PREBUILT:-}" ]; then
  echo "KREUZBERG_PDFIUM_PREBUILT is not set" >&2
  exit 1
fi

case "${RUNNER_OS:-unknown}" in
Windows)
  src_path="$KREUZBERG_PDFIUM_PREBUILT/bin/pdfium.dll"
  filename="pdfium.dll"
  lib_dir="$KREUZBERG_PDFIUM_PREBUILT/lib"
  if [ -f "$lib_dir/pdfium.dll.lib" ]; then
    cp -f "$lib_dir/pdfium.dll.lib" "$lib_dir/pdfium.lib"
  fi
  ;;
macOS)
  src_path="$KREUZBERG_PDFIUM_PREBUILT/lib/libpdfium.dylib"
  filename="libpdfium.dylib"
  ;;
Linux)
  src_path="$KREUZBERG_PDFIUM_PREBUILT/lib/libpdfium.so"
  filename="libpdfium.so"
  ;;
*)
  echo "Unsupported RUNNER_OS: ${RUNNER_OS:-unknown}" >&2
  exit 1
  ;;
esac

mkdir -p "$destination"
cp -f "$src_path" "${destination}/$filename"

if [ -n "$additional_destinations" ]; then
  while IFS= read -r dest; do
    if [ -n "$dest" ]; then
      mkdir -p "$dest"
      cp -f "$src_path" "$dest/$filename"
    fi
  done <<<"$additional_destinations"
fi

if [ "${RUNNER_OS:-unknown}" = "Windows" ]; then
  echo "$GITHUB_WORKSPACE/$destination" >>"$GITHUB_PATH"
fi
