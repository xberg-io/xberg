#!/bin/bash
set -e

# Install the kreuzberg PHP extension to the system PHP extension directory
# Called from the before hook in alef.toml for PHP e2e tests

EXTENSION_DIR=$(php -r 'echo ini_get("extension_dir");')

# Find the built extension
for path in target/release/libkreuzberg_php.dylib target/release/libkreuzberg_php.so target/release/kreuzberg_php.dll; do
  if [ -f "$path" ]; then
    EXT_PATH="$path"
    break
  fi
done

if [ -z "$EXT_PATH" ]; then
  echo "Error: PHP extension not found in target/release/" >&2
  exit 1
fi

# Copy to extension directory
EXT_FILENAME=$(basename "$EXT_PATH")
cp "$EXT_PATH" "$EXTENSION_DIR/$EXT_FILENAME"

# Add to php.ini if not already present
PHP_INI=$(php -r 'echo php_ini_loaded_file();')
if ! grep -q "extension=$EXT_FILENAME" "$PHP_INI"; then
  echo "extension=$EXT_FILENAME" >>"$PHP_INI"
fi

echo "Installed PHP extension: $EXT_FILENAME to $EXTENSION_DIR"
