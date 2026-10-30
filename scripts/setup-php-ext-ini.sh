#!/bin/bash
set -e

EXT_DIR=$(php -r 'echo ini_get("extension_dir");')

for path in ../../target/release/libxberg_php.dylib ../../target/release/libxberg_php.so ../../target/release/xberg_php.dll; do
  if [ -f "$path" ]; then
    BUILT_EXT="$path"
    break
  fi
done

if [ -z "$BUILT_EXT" ]; then
  echo "Error: xberg PHP extension not found in target/release/" >&2
  exit 1
fi

BUILT_EXT=$(cd "$(dirname "$BUILT_EXT")" && pwd)/$(basename "$BUILT_EXT")

BASENAME=$(basename "$BUILT_EXT")
TARGET="$EXT_DIR/$BASENAME"
cp "$BUILT_EXT" "$TARGET"
echo "Extension copied/verified: $TARGET"

cat >php.ini <<EOF
; Temporary PHP INI for e2e tests — loads xberg PHP extension from system extension directory
[PHP]
extension_dir=$EXT_DIR
extension=$BASENAME
EOF

echo "Created php.ini that loads: $BASENAME"
