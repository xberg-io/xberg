#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?usage: $0 <target> <platform> <version> <output-dir>}"
PLATFORM="${2:?usage: $0 <target> <platform> <version> <output-dir>}"
VERSION="${3:?usage: $0 <target> <platform> <version> <output-dir>}"
OUTPUT_DIR="${4:?usage: $0 <target> <platform> <version> <output-dir>}"

case "$TARGET:$PLATFORM" in
x86_64-unknown-linux-musl:linux-x86_64-musl | aarch64-unknown-linux-musl:linux-aarch64-musl) ;;
*)
  echo "unsupported Go musl release target/platform: $TARGET / $PLATFORM" >&2
  exit 2
  ;;
esac

BUILD_ID="$(git rev-parse HEAD)"
[ -n "$BUILD_ID" ] || {
  echo "git rev-parse HEAD produced an empty build id" >&2
  exit 1
}

BUILD_OUTPUT="$(mktemp -d)"
[ -n "$BUILD_OUTPUT" ] && [ -d "$BUILD_OUTPUT" ] || exit 90
STAGE="$OUTPUT_DIR/xberg-go-v$VERSION-$PLATFORM"
trap 'rm -rf "$BUILD_OUTPUT" "$STAGE"' EXIT

docker build -f docker/Dockerfile.musl-ffi \
  --build-arg "XBERG_BUILD_ID=$BUILD_ID" \
  --output "type=local,dest=$BUILD_OUTPUT" .

mkdir -p "$STAGE/lib" "$STAGE/include"
cp "$BUILD_OUTPUT"/*.so* "$STAGE/lib/"
cp crates/xberg-ffi/include/xberg.h "$STAGE/include/"

ARCHIVE="$OUTPUT_DIR/xberg-go-v$VERSION-$PLATFORM.tar.gz"
tar -czf "$ARCHIVE" -C "$OUTPUT_DIR" "$(basename "$STAGE")"

CONTENTS="$(tar -tzf "$ARCHIVE")"
grep -q "/lib/libxberg_ffi.so$" <<<"$CONTENTS"
grep -q "/include/xberg.h$" <<<"$CONTENTS"
