#!/bin/bash

set -e

echo "Building Kreuzberg PHP extension..."
echo "===================================="
echo ""

command -v cargo >/dev/null 2>&1 || {
	echo "Error: cargo not found. Please install Rust from https://rustup.rs/"
	exit 1
}

command -v php >/dev/null 2>&1 || {
	echo "Error: php not found. Please install PHP 8.0 or later."
	exit 1
}

command -v clang >/dev/null 2>&1 || {
	echo "Warning: clang not found. ext-php-rs requires clang for building."
	echo "Please install clang for your platform."
}

echo "Tool versions:"
echo "  Rust:  $(rustc --version)"
echo "  Cargo: $(cargo --version)"
echo "  PHP:   $(php --version | head -n1)"
echo ""

echo "Compiling in release mode..."
cargo build --release

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
	EXT_FILE="libkreuzberg.so"
	PHP_EXT="kreuzberg.so"
elif [[ "$OSTYPE" == "darwin"* ]]; then
	EXT_FILE="libkreuzberg.dylib"
	PHP_EXT="kreuzberg.so"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
	EXT_FILE="kreuzberg.dll"
	PHP_EXT="kreuzberg.dll"
else
	echo "Warning: Unknown OS type: $OSTYPE"
	EXT_FILE="libkreuzberg.so"
	PHP_EXT="kreuzberg.so"
fi

BUILT_LIB="../../target/release/$EXT_FILE"

if [ ! -f "$BUILT_LIB" ]; then
	echo "Error: Built library not found at $BUILT_LIB"
	exit 1
fi

echo ""
echo "Build successful!"
echo "================="
echo ""
echo "Extension location: $BUILT_LIB"
echo ""
echo "To install:"
echo "1. Copy the extension to your PHP extension directory:"
echo "   sudo cp $BUILT_LIB \$(php-config --extension-dir)/$PHP_EXT"
echo ""
echo "2. Add to your php.ini:"
echo "   extension=$PHP_EXT"
echo ""
echo "3. Verify installation:"
echo "   php -m | grep kreuzberg"
echo ""
echo "For development, you can also use:"
echo "   php -d extension=$BUILT_LIB examples/basic_usage.php"
echo ""
