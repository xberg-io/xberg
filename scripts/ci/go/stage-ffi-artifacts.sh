#!/usr/bin/env bash
set -euo pipefail

STAGING_DIR="${1:-artifact-staging/kreuzberg-ffi}"
export BUILD_FEATURES="${2:-default}"

echo "=== Staging FFI artifacts to ${STAGING_DIR} ==="

shopt -s nullglob

ffi_libs=(target/release/libkreuzberg_ffi.*)
if [ ${#ffi_libs[@]} -eq 0 ]; then
	echo "ERROR: No FFI library found in target/release/" >&2
	exit 1
fi
cp "${ffi_libs[@]}" "${STAGING_DIR}/lib/"
echo "✓ Staged FFI library: ${ffi_libs[*]}"

pdfium_libs=(target/release/libpdfium.*)
if [ ${#pdfium_libs[@]} -gt 0 ]; then
	cp "${pdfium_libs[@]}" "${STAGING_DIR}/lib/"
	echo "✓ Staged PDFium library: ${pdfium_libs[*]}"
fi

shopt -u nullglob

cp crates/kreuzberg-ffi/kreuzberg.h "${STAGING_DIR}/include/"

cp crates/kreuzberg-ffi/kreuzberg-ffi-install.pc "${STAGING_DIR}/share/pkgconfig/kreuzberg-ffi.pc"

echo "✓ FFI artifacts staged successfully"
