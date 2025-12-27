#!/usr/bin/env bash
set -euo pipefail

rid="${1:?rid required}"
out="${2:-java-natives/${rid}}"

mkdir -p "$out"

case "$rid" in
windows-x86_64)
	cp -f target/release/kreuzberg_ffi.dll "$out/"
	if [ -f target/release/pdfium.dll ]; then
		cp -f target/release/pdfium.dll "$out/"
	fi
	;;
macos-x86_64 | macos-arm64)
	cp -f target/release/libkreuzberg_ffi.dylib "$out/"
	if [ -f target/release/libpdfium.dylib ]; then
		cp -f target/release/libpdfium.dylib "$out/"
	fi
	;;
linux-x86_64)
	cp -f target/release/libkreuzberg_ffi.so "$out/"
	if [ -f target/release/libpdfium.so ]; then
		cp -f target/release/libpdfium.so "$out/"
	fi
	;;
*)
	echo "Unsupported rid: $rid" >&2
	exit 1
	;;
esac

ls -la "$out"
