#!/usr/bin/env bash

set -euo pipefail

pkg="$(find artifacts/csharp -maxdepth 1 -name '*.nupkg' -print | sort | head -n 1)"
if [ -z "$pkg" ]; then
	echo "No .nupkg found under artifacts/csharp" >&2
	exit 1
fi

echo "Verifying native assets in: $pkg"
pkg_size=$(find "$pkg" -maxdepth 0 -exec stat -f%z {} \; 2>/dev/null || find "$pkg" -maxdepth 0 -exec stat -c%s {} \;)
echo "Package size: $((pkg_size / 1024))K"

echo ""
echo "=== All runtimes in package ==="
unzip -l "$pkg" | grep "runtimes/" | head -20 || echo "  (no runtimes found)"

missing_files=0
for rid in linux-x64 osx-arm64 win-x64; do
	echo ""
	echo "Checking $rid..."

	if unzip -l "$pkg" | grep -E "runtimes/${rid}/native/.*kreuzberg_ffi\\.(dll|so|dylib)"; then
		echo "  ✓ Found kreuzberg_ffi for $rid"
	else
		echo "  ✗ Missing kreuzberg_ffi binary for $rid" >&2
		missing_files=$((missing_files + 1))
	fi

done

if [ "$missing_files" -gt 0 ]; then
	echo ""
	echo "::error::Missing $missing_files native asset(s) in NuGet package" >&2
	exit 1
fi

echo ""
echo "All native assets verified successfully"
