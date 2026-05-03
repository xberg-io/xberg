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
echo "=== Checking static library (required for Go) ==="
STATIC_LIB="verify-temp/kreuzberg-ffi/lib/libkreuzberg_ffi.a"
if [ -f "$STATIC_LIB" ]; then
  echo "✓ Found static library: libkreuzberg_ffi.a ($(du -h "$STATIC_LIB" | cut -f1))"
else
  echo "✗ Missing static library: libkreuzberg_ffi.a"
  exit 1
fi

echo ""
echo "=== Checking platform-specific dynamic libraries (optional) ==="
PLATFORM_LIBS_FOUND=0

if find verify-temp/kreuzberg-ffi/lib -name "*.so" -o -name "*.so.*" 2>/dev/null | grep -q .; then
  LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.so*" 2>/dev/null | head -1)
  if [ -n "$LIBKREUZBERG" ]; then
    echo "✓ Found Linux dynamic library: $(basename "$LIBKREUZBERG")"
    PLATFORM_LIBS_FOUND=1
  fi
fi

if find verify-temp/kreuzberg-ffi/lib -name "*.dylib" 2>/dev/null | grep -q .; then
  LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.dylib" 2>/dev/null | head -1)
  if [ -n "$LIBKREUZBERG" ]; then
    echo "✓ Found macOS dynamic library: $(basename "$LIBKREUZBERG")"
    PLATFORM_LIBS_FOUND=1
  fi
fi

if find verify-temp/kreuzberg-ffi/lib -name "*.dll" 2>/dev/null | grep -q .; then
  LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "kreuzberg_ffi.dll" 2>/dev/null | head -1)
  if [ -n "$LIBKREUZBERG" ]; then
    echo "✓ Found Windows dynamic library: $(basename "$LIBKREUZBERG")"
    PLATFORM_LIBS_FOUND=1
  fi
fi

if [ $PLATFORM_LIBS_FOUND -eq 0 ]; then
  echo "  (No dynamic libraries found - static linking only)"
fi

echo ""
echo "=== Checking required FFI symbols ==="
# These symbols are declared in packages/go/v4/binding.go and must be exported
# by the compiled library. A missing symbol causes linker errors at Go build
# time (regression: v4.8.0–v4.9.4, tracked in issue #871).
REQUIRED_SYMBOLS=(
  "kreuzberg_embed_texts"
  "kreuzberg_get_embedding_preset"
  "kreuzberg_list_embedding_presets"
)

if command -v nm >/dev/null 2>&1; then
  SYMBOL_ERRORS=0
  for sym in "${REQUIRED_SYMBOLS[@]}"; do
    if nm "$STATIC_LIB" 2>/dev/null | grep -qF "$sym"; then
      echo "✓ Symbol present: $sym"
    else
      echo "✗ Missing symbol: $sym"
      SYMBOL_ERRORS=$((SYMBOL_ERRORS + 1))
    fi
  done
  if [ $SYMBOL_ERRORS -gt 0 ]; then
    echo ""
    echo "✗ $SYMBOL_ERRORS required symbol(s) missing from libkreuzberg_ffi.a"
    exit 1
  fi
else
  echo "  (nm not available — skipping symbol check)"
fi

echo ""
echo "✓ Artifact verification passed"
