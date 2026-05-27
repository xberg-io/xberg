#!/bin/bash
set -e

# Setup temporary php.ini for e2e/php that loads the kreuzberg extension from target/release
# Called from alef.toml before hook for PHP e2e tests
# Must be run from e2e/php directory

EXT_DIR=$(php -r 'echo ini_get("extension_dir");')

# Look for built extension (relative to e2e/php/)
for path in ../../target/release/libkreuzberg_php.dylib ../../target/release/libkreuzberg_php.so ../../target/release/kreuzberg_php.dll; do
  if [ -f "$path" ]; then
    BUILT_EXT="$path"
    break
  fi
done

if [ -z "$BUILT_EXT" ]; then
  echo "Error: kreuzberg PHP extension not found in target/release/" >&2
  exit 1
fi

# Resolve to absolute path
BUILT_EXT=$(cd "$(dirname "$BUILT_EXT")" && pwd)/$(basename "$BUILT_EXT")

# Copy extension to extension directory
BASENAME=$(basename "$BUILT_EXT")
TARGET="$EXT_DIR/$BASENAME"
cp "$BUILT_EXT" "$TARGET" 2>/dev/null || true # May fail if already exists, that's OK
echo "Extension copied/verified: $TARGET"

# Create php.ini in current directory (e2e/php) that loads the extension.
# extension_dir is set explicitly so the ini works even when invoked with
# PHP_INI_SCAN_DIR= (which is recommended in the e2e runner to skip stale
# conf.d/*.ini entries left behind by sibling projects).
cat >php.ini <<EOF
; Temporary PHP INI for e2e tests — loads kreuzberg PHP extension from system extension directory
[PHP]
extension_dir=$EXT_DIR
extension=$BASENAME
EOF

echo "Created php.ini that loads: $BASENAME"
