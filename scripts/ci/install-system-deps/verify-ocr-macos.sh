#!/usr/bin/env bash
set -euo pipefail

verify_ocr_macos() {
  local tess_prefix lept_prefix output
  tess_prefix="$(brew --prefix tesseract)"
  lept_prefix="$(brew --prefix leptonica)"
  export PKG_CONFIG_PATH="$tess_prefix/lib/pkgconfig:$lept_prefix/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
  [ "$(pkg-config --modversion tesseract)" = 5.5.5 ] || {
    echo 'Expected Tesseract pkg-config 5.5.5' >&2
    return 1
  }
  [ "$(pkg-config --modversion lept)" = 1.87.0 ] || {
    echo 'Expected Leptonica pkg-config 1.87.0' >&2
    return 1
  }
  output="$(tesseract --version 2>&1)"
  grep -qx 'tesseract 5\.5\.5' <<<"$output" || {
    echo 'Expected actual Tesseract CLI 5.5.5' >&2
    return 1
  }
  grep -q 'leptonica-1\.87\.0\([^0-9.]\|$\)' <<<"$output" || {
    echo 'Expected actual Leptonica runtime 1.87.0' >&2
    return 1
  }
  printf '%s\n' "$output"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  verify_ocr_macos
fi
