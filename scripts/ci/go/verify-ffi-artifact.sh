#!/usr/bin/env bash
set -euo pipefail

ARTIFACT="${1}"

if [ ! -f "${ARTIFACT}" ]; then
	echo "✗ Artifact not found: ${ARTIFACT}"
	exit 1
fi

echo "=== Verifying artifact structure ==="
tar -tzf "${ARTIFACT}"

cleanup() {
	rm -rf verify-temp
}
trap cleanup EXIT

mkdir -p verify-temp
tar -xzf "${ARTIFACT}" -C verify-temp

REQUIRED_FILES=(
	"kreuzberg-ffi/include/kreuzberg.h"
	"kreuzberg-ffi/share/pkgconfig/kreuzberg-ffi.pc"
)

echo ""
echo "=== Checking required files ==="
for file in "${REQUIRED_FILES[@]}"; do
	if [ -f "verify-temp/$file" ]; then
		echo "✓ Found: $file"
	else
		echo "✗ Missing: $file"
		exit 1
	fi
done

echo ""
echo "=== Checking platform-specific libraries ==="
PLATFORM_LIBS_FOUND=0

if find verify-temp/kreuzberg-ffi/lib -name "*.so" -o -name "*.so.*" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.so*" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found Linux library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

if find verify-temp/kreuzberg-ffi/lib -name "*.dylib" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.dylib" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found macOS library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

if find verify-temp/kreuzberg-ffi/lib -name "*.dll" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "kreuzberg_ffi.dll" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found Windows library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

if [ $PLATFORM_LIBS_FOUND -eq 0 ]; then
	echo "✗ No platform libraries found (expected .so, .dylib, or .dll)"
	exit 1
fi

echo ""
echo "✓ Artifact verification passed"
