#!/usr/bin/env bash
set -euo pipefail

OCR_PREFIX="${OCR_PREFIX:-/usr/local}"
OCR_BUILD_JOBS="${OCR_BUILD_JOBS:-${CMAKE_BUILD_PARALLEL_LEVEL:-1}}"
OCR_TESSDATA_DIR="${OCR_TESSDATA_DIR:-/usr/share/tesseract-ocr/5/tessdata}"
export PKG_CONFIG_PATH="$OCR_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
export LD_LIBRARY_PATH="$OCR_PREFIX/lib:${LD_LIBRARY_PATH:-}"

ocr_privileged() {
  if [ "$EUID" -eq 0 ]; then "$@"; else sudo "$@"; fi
}

ocr_versions_match() {
  local output
  [ "$(pkg-config --modversion tesseract 2>/dev/null)" = 5.5.5 ] || return 1
  [ "$(pkg-config --modversion lept 2>/dev/null)" = 1.87.0 ] || return 1
  [ -x "$OCR_PREFIX/bin/tesseract" ] || return 1
  output="$("$OCR_PREFIX/bin/tesseract" --version 2>&1)" || return 1
  grep -qx 'tesseract 5\.5\.5' <<<"$output" &&
    grep -q 'leptonica-1\.87\.0\([^0-9.]\|$\)' <<<"$output"
}

ocr_assert_versions() {
  if ! ocr_versions_match; then
    echo '::error::expected Tesseract 5.5.5 and Leptonica 1.87.0 CLI/pkg-config runtimes' >&2
    return 1
  fi
  "$OCR_PREFIX/bin/tesseract" --version
  pkg-config --modversion tesseract lept
}

ocr_build_source() {
  local name="$1" version="$2" url="$3" digest="$4"
  shift 4
  local archive="$work_dir/$name.tar.gz"
  curl --retry 3 --max-time 300 -fsSL -o "$archive" "$url"
  printf '%s  %s\n' "$digest" "$archive" | sha256sum -c -
  tar -xzf "$archive" -C "$work_dir"
  cmake -S "$work_dir/$name-$version" -B "$work_dir/$name-build" \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$OCR_PREFIX" \
    -DCMAKE_INSTALL_LIBDIR=lib -DBUILD_SHARED_LIBS=ON "$@"
  cmake --build "$work_dir/$name-build" --parallel "$OCR_BUILD_JOBS"
  ocr_privileged cmake --install "$work_dir/$name-build"
}

ocr_verify_runtime() {
  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  ocr_assert_versions
  python3 "$script_dir/verify-ocr-linux.py" "$OCR_PREFIX"
  [ -s "$OCR_TESSDATA_DIR/eng.traineddata" ] || {
    echo 'Missing existing English tessdata' >&2
    return 1
  }
  "$OCR_PREFIX/bin/tesseract" --tessdata-dir "$OCR_TESSDATA_DIR" --list-langs >"$work_dir/languages"
  grep -qx eng "$work_dir/languages"
  cat "$work_dir/languages"
  bash "$script_dir/verify-ocr-fixture.sh" "$OCR_PREFIX/bin/tesseract" "$OCR_TESSDATA_DIR"
}

ocr_install_licenses() {
  local destination="$OCR_PREFIX/share/licenses/xberg-system-ocr"
  ocr_privileged install -d "$destination/tesseract-5.5.5" "$destination/leptonica-1.87.0"
  ocr_privileged install -m 644 "$work_dir/tesseract-5.5.5/LICENSE" "$work_dir/tesseract-5.5.5/AUTHORS" "$destination/tesseract-5.5.5/"
  ocr_privileged install -m 644 "$work_dir/leptonica-1.87.0/leptonica-license.txt" "$destination/leptonica-1.87.0/"
}

ocr_main() {
  [[ "$OCR_BUILD_JOBS" =~ ^[1-9][0-9]*$ ]] || {
    echo 'OCR_BUILD_JOBS must be positive' >&2
    return 1
  }
  work_dir="$(mktemp -d)"
  [ -n "$work_dir" ] && [ -d "$work_dir" ] || return 1
  trap 'rm -rf "$work_dir"' EXIT
  if ! ocr_versions_match ||
    [ ! -s "$OCR_PREFIX/share/licenses/xberg-system-ocr/tesseract-5.5.5/LICENSE" ] ||
    [ ! -s "$OCR_PREFIX/share/licenses/xberg-system-ocr/tesseract-5.5.5/AUTHORS" ] ||
    [ ! -s "$OCR_PREFIX/share/licenses/xberg-system-ocr/leptonica-1.87.0/leptonica-license.txt" ]; then
    ocr_build_source leptonica 1.87.0 \
      https://github.com/DanBloomberg/leptonica/releases/download/1.87.0/leptonica-1.87.0.tar.gz \
      c73363397f96eb1295602bf44d708a994ad42046c791bf03ea0505d829bdb6a7 \
      -DBUILD_PROG=OFF -DSW_BUILD=OFF -DSTRICT_CONF=ON
    ocr_build_source tesseract 5.5.5 \
      https://github.com/tesseract-ocr/tesseract/archive/refs/tags/5.5.5.tar.gz \
      b02cf6f37cfc95be5bf7e4b2d39e51997b1ac41f7beb4e2bd16777bb67561c79 \
      -DLeptonica_DIR="$OCR_PREFIX/lib/cmake/leptonica" \
      -DTESSDATA_PREFIX="$(dirname "$OCR_TESSDATA_DIR")" \
      -DBUILD_TRAINING_TOOLS=OFF -DBUILD_TESTS=OFF -DOPENMP_BUILD=ON
    ocr_install_licenses
  fi
  ocr_privileged install -d /etc/ld.so.conf.d
  printf '%s\n' "$OCR_PREFIX/lib" | ocr_privileged tee /etc/ld.so.conf.d/xberg-system-ocr.conf >/dev/null
  ocr_privileged ldconfig
  ocr_verify_runtime
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then ocr_main; fi
