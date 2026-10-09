#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?usage: $0 <target> <version> <output-dir>}"
VERSION="${2:?usage: $0 <target> <version> <output-dir>}"
OUTPUT_DIR="${3:?usage: $0 <target> <version> <output-dir>}"

case "$TARGET" in
x86_64-pc-windows-msvc)
  SHARED_NAME="xberg_ffi.dll"
  STATIC_NAME="xberg_ffi.lib"
  ;;
*-apple-darwin)
  SHARED_NAME="libxberg_ffi.dylib"
  STATIC_NAME="libxberg_ffi.a"
  ;;
*-unknown-linux-gnu)
  SHARED_NAME="libxberg_ffi.so"
  STATIC_NAME="libxberg_ffi.a"
  ;;
*)
  echo "unsupported Go release target: $TARGET" >&2
  exit 2
  ;;
esac

RELEASE_DIR="target/$TARGET/release"
SHARED_PATH="$RELEASE_DIR/$SHARED_NAME"

if [[ "$TARGET" == *-unknown-linux-gnu ]]; then
  GLIBC_FLOOR="${GLIBC_FLOOR:?GLIBC_FLOOR is required for Linux Go release packaging}"
  cargo zigbuild --locked -p xberg-ffi --release --target "$TARGET.$GLIBC_FLOOR"
  [ -f "$SHARED_PATH" ] || {
    echo "glibc-floor shared library not found: $SHARED_PATH" >&2
    exit 1
  }
  [ -f "$RELEASE_DIR/$STATIC_NAME" ] || {
    echo "glibc-floor static library not found: $RELEASE_DIR/$STATIC_NAME" >&2
    exit 1
  }
  FLOOR_ARTIFACTS="$(mktemp -d)"
  [ -n "$FLOOR_ARTIFACTS" ] && [ -d "$FLOOR_ARTIFACTS" ] || exit 90
  trap 'rm -rf "$FLOOR_ARTIFACTS"' EXIT
  cp "$SHARED_PATH" "$RELEASE_DIR/$STATIC_NAME" "$FLOOR_ARTIFACTS/"
fi

alef publish build --lang go --target "$TARGET"
if [[ "$TARGET" == *-unknown-linux-gnu ]]; then
  cp "$FLOOR_ARTIFACTS/$SHARED_NAME" "$SHARED_PATH"
  cp "$FLOOR_ARTIFACTS/$STATIC_NAME" "$RELEASE_DIR/$STATIC_NAME"
fi
[ -f "$SHARED_PATH" ] || {
  echo "shared library not found: $SHARED_PATH" >&2
  exit 1
}
alef publish package --lang go --target "$TARGET" --version "$VERSION" --output "$OUTPUT_DIR"

PLATFORM=""
case "$TARGET" in
x86_64-unknown-linux-gnu) PLATFORM="linux-x86_64" ;;
aarch64-unknown-linux-gnu) PLATFORM="linux-aarch64" ;;
aarch64-apple-darwin) PLATFORM="macos-arm64" ;;
x86_64-apple-darwin) PLATFORM="macos-x86_64" ;;
x86_64-pc-windows-msvc) PLATFORM="windows-x86_64" ;;
esac
ARCHIVE="$OUTPUT_DIR/xberg-go-v$VERSION-$PLATFORM.tar.gz"
[ -f "$ARCHIVE" ] || {
  echo "Go release archive not found: $ARCHIVE" >&2
  exit 1
}

CONTENTS="$(tar -tzf "$ARCHIVE")"
grep -q "/lib/$SHARED_NAME$" <<<"$CONTENTS"
grep -q "/lib/$STATIC_NAME$" <<<"$CONTENTS"
grep -q "/lib/native-static-libs.txt$" <<<"$CONTENTS"
