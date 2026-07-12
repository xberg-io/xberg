#!/usr/bin/env bash
# Build the libheif decode stack (libde265, libaom, libheif) from source at
# MACOSX_DEPLOYMENT_TARGET, into a prefix that macOS artifacts bundle instead
# of Homebrew bottles, which target the runner's own macOS (#1243).
# Decode-only, mirroring the manylinux build-libheif step in xberg-io/actions.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
source "$REPO_ROOT/scripts/lib/retry.sh"
source "$REPO_ROOT/scripts/lib/macho.sh"

LIBDE265_VERSION=1.0.16
LIBAOM_VERSION=3.12.1
LIBHEIF_VERSION=1.23.0 # keep in step with build-libheif in xberg-io/actions

PREFIX="${XBERG_HEIF_PREFIX:-/tmp/xberg-heif}"
TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
WORK="$(mktemp -d)"
JOBS="$(sysctl -n hw.ncpu)"

# Absolute install names so delocate can resolve every load command.
CMAKE_COMMON=(
  -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_INSTALL_PREFIX="$PREFIX"
  -DCMAKE_INSTALL_LIBDIR=lib
  -DCMAKE_INSTALL_NAME_DIR="$PREFIX/lib"
  -DCMAKE_OSX_DEPLOYMENT_TARGET="$TARGET"
  -DBUILD_SHARED_LIBS=ON
)

ensure_nasm() {
  [ "$(uname -m)" = "x86_64" ] || return 0
  command -v nasm >/dev/null 2>&1 || retry_with_backoff brew install nasm
}

build_dep() {
  local name="$1" version="$2" url="$3"
  shift 3
  echo "::group::$name $version"
  retry_with_backoff curl -fsSL --retry 3 -o "$WORK/$name.tar.gz" "$url"
  tar -xzf "$WORK/$name.tar.gz" -C "$WORK"
  cmake -S "$WORK/$name-$version" -B "$WORK/$name-build" "${CMAKE_COMMON[@]}" "$@"
  cmake --build "$WORK/$name-build" -j "$JOBS"
  cmake --install "$WORK/$name-build"
  echo "::endgroup::"
}

deps_of() {
  otool -L "$1" | tail -n +3 | sed 's/^[[:space:]]*//; s/ (compatibility.*//'
}

verify_dylib() {
  local lib="$1" minos dep
  minos="$(minos_of "$lib")"
  if [ "$minos" != "$TARGET" ]; then
    echo "::error::$(basename "$lib") targets macOS $minos, expected $TARGET"
    return 1
  fi
  while IFS= read -r dep; do
    case "$dep" in
      "$PREFIX"/lib/* | /usr/lib/* | /System/*) ;;
      *)
        echo "::error::$(basename "$lib") links outside the prefix: $dep"
        return 1
        ;;
    esac
  done < <(deps_of "$lib")
}

verify_prefix() {
  local lib fail=0
  for lib in "$PREFIX"/lib/*.dylib; do
    [ -L "$lib" ] && continue
    verify_dylib "$lib" || fail=1
  done
  return "$fail"
}

main() {
  if [ -f "$PREFIX/lib/libheif.dylib" ]; then
    echo "libheif stack already present in $PREFIX, skipping build"
    return 0
  fi
  trap 'rm -rf "$WORK"' EXIT
  ensure_nasm

  build_dep libde265 "$LIBDE265_VERSION" \
    "https://github.com/strukturag/libde265/releases/download/v$LIBDE265_VERSION/libde265-$LIBDE265_VERSION.tar.gz" \
    -DENABLE_SDL=OFF

  build_dep libaom "$LIBAOM_VERSION" \
    "https://storage.googleapis.com/aom-releases/libaom-$LIBAOM_VERSION.tar.gz" \
    -DCONFIG_AV1_ENCODER=0 -DENABLE_EXAMPLES=0 -DENABLE_TESTS=0 -DENABLE_DOCS=0 -DENABLE_TOOLS=0

  # Pin dependency discovery to this prefix so no Homebrew library leaks in.
  PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig" build_dep libheif "$LIBHEIF_VERSION" \
    "https://github.com/strukturag/libheif/releases/download/v$LIBHEIF_VERSION/libheif-$LIBHEIF_VERSION.tar.gz" \
    -DCMAKE_PREFIX_PATH="$PREFIX" -DCMAKE_IGNORE_PREFIX_PATH="/opt/homebrew;/usr/local" \
    -DWITH_LIBDE265=ON -DWITH_AOM_DECODER=ON -DWITH_AOM_ENCODER=OFF -DWITH_X265=OFF \
    -DWITH_EXAMPLES=OFF -DWITH_GDK_PIXBUF=OFF -DBUILD_TESTING=OFF

  verify_prefix
  echo "libheif stack built for macOS $TARGET in $PREFIX"
}

main "$@"
