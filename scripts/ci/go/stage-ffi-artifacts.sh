#!/usr/bin/env bash
set -euo pipefail

STAGING_DIR="${1:-artifact-staging/kreuzberg-ffi}"
export BUILD_FEATURES="${2:-default}"

echo "=== Staging FFI artifacts to ${STAGING_DIR} ==="

shopt -s nullglob

# Stage static library (.a) - required for Go static linking
static_lib="target/release/libkreuzberg_ffi.a"
if [ -f "$static_lib" ]; then
  cp "$static_lib" "${STAGING_DIR}/lib/"
  echo "✓ Staged static library: $static_lib ($(du -h "$static_lib" | cut -f1))"
else
  echo "ERROR: Static library not found: $static_lib" >&2
  exit 1
fi

# Stage dynamic libraries (.so, .dylib, .dll) - optional for runtime linking
ffi_libs=(target/release/libkreuzberg_ffi.{so,dylib,dll} target/release/libkreuzberg_ffi.so.*)
ffi_libs_found=()
for lib in "${ffi_libs[@]}"; do
  if [ -f "$lib" ]; then
    cp "$lib" "${STAGING_DIR}/lib/"
    ffi_libs_found+=("$lib")
  fi
done
if [ ${#ffi_libs_found[@]} -gt 0 ]; then
  echo "✓ Staged dynamic libraries: ${ffi_libs_found[*]}"
fi

# Stage PDFium libraries
pdfium_libs=(target/release/libpdfium.*)
if [ ${#pdfium_libs[@]} -gt 0 ]; then
  cp "${pdfium_libs[@]}" "${STAGING_DIR}/lib/"
  echo "✓ Staged PDFium library: ${pdfium_libs[*]}"
fi

shopt -u nullglob

# Stage header file
cp crates/kreuzberg-ffi/include/kreuzberg.h "${STAGING_DIR}/include/"
echo "✓ Staged header: kreuzberg.h"

# Stage pkg-config file (generated inline — the .pc carries the version and is gitignored).
ffi_version="$(grep -m1 '^version' crates/kreuzberg-ffi/Cargo.toml | cut -d '"' -f2)"
cat > "${STAGING_DIR}/share/pkgconfig/kreuzberg-ffi.pc" <<EOF
prefix=/usr/local
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: kreuzberg-ffi
Description: C FFI bindings for Kreuzberg document intelligence library
Version: ${ffi_version}
URL: https://kreuzberg.dev
Libs: -L\${libdir} -lkreuzberg_ffi
Cflags: -I\${includedir}
EOF
echo "✓ Staged pkg-config: kreuzberg-ffi.pc (version=${ffi_version})"

echo ""
echo "✓ FFI artifacts staged successfully to ${STAGING_DIR}"
echo "  Contents:"
ls -la "${STAGING_DIR}/lib/" 2>/dev/null || true
