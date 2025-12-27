#!/usr/bin/env bash

set -euo pipefail

target="${TARGET:?TARGET not set}"

mkdir -p crates/kreuzberg-node/artifacts
pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts
if [ ! -d crates/kreuzberg-node/npm ]; then
	echo "npm artifact directory missing" >&2
	exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
	echo "jq is required to package Node runtime dependencies" >&2
	exit 1
fi

case "$target" in
aarch64-apple-darwin)
	platform_dir="darwin-arm64"
	node_file="kreuzberg-node.darwin-arm64.node"
	pdfium_file="libpdfium.dylib"
	;;
x86_64-apple-darwin)
	platform_dir="darwin-x64"
	node_file="kreuzberg-node.darwin-x64.node"
	pdfium_file="libpdfium.dylib"
	;;
x86_64-pc-windows-msvc)
	platform_dir="win32-x64-msvc"
	node_file="kreuzberg-node.win32-x64-msvc.node"
	pdfium_file="pdfium.dll"
	;;
aarch64-pc-windows-msvc)
	platform_dir="win32-arm64-msvc"
	node_file="kreuzberg-node.win32-arm64-msvc.node"
	pdfium_file="pdfium.dll"
	;;
x86_64-unknown-linux-gnu)
	platform_dir="linux-x64-gnu"
	node_file="kreuzberg-node.linux-x64-gnu.node"
	pdfium_file="libpdfium.so"
	;;
aarch64-unknown-linux-gnu)
	platform_dir="linux-arm64-gnu"
	node_file="kreuzberg-node.linux-arm64-gnu.node"
	pdfium_file="libpdfium.so"
	;;
armv7-unknown-linux-gnueabihf)
	platform_dir="linux-arm-gnueabihf"
	node_file="kreuzberg-node.linux-arm-gnueabihf.node"
	pdfium_file="libpdfium.so"
	;;
*)
	echo "Unsupported NAPI target: $target" >&2
	exit 1
	;;
esac

dest="crates/kreuzberg-node/npm/${platform_dir}/${node_file}"
src=""

echo ""
echo "=========================================="
echo "Package Artifacts for Target: $target"
echo "=========================================="
echo "Platform directory: $platform_dir"
echo "Expected node file: $node_file"
echo "Destination: $dest"
echo ""

echo "Looking for NAPI binary: ${node_file} (platform: ${platform_dir}, target: ${target})"

for candidate in "crates/kreuzberg-node/artifacts/${node_file}" "crates/kreuzberg-node/${node_file}"; do
	echo "  Checking: $candidate"
	if [ -f "$candidate" ]; then
		src="$candidate"
		echo "  ✓ Found: $src"
		ls -lh "$src"
		break
	else
		echo "  ✗ Not found"
	fi
done

if [ -z "$src" ]; then
	echo ""
	echo "::error::Missing built NAPI binary: expected ${node_file}" >&2
	echo ""
	echo "Expected locations:" >&2
	echo "  - crates/kreuzberg-node/artifacts/${node_file}" >&2
	echo "  - crates/kreuzberg-node/${node_file}" >&2
	echo ""
	echo "Available .node files:" >&2
	find crates/kreuzberg-node -maxdepth 3 -type f -name "*.node" -print 2>/dev/null || echo "  (none found)"
	echo ""
	echo "npm directory structure:" >&2
	find crates/kreuzberg-node/npm -type d 2>/dev/null | head -20 || echo "  (npm directory not created)"
	echo ""
	echo "Full crates/kreuzberg-node directory:" >&2
	find crates/kreuzberg-node -type f \( -name "*.node" -o -name "package.json" \) | head -30
	exit 1
fi

platform_npm_dir="crates/kreuzberg-node/npm/${platform_dir}"
echo ""
echo "Ensuring platform directory exists: $platform_npm_dir"
mkdir -p "$platform_npm_dir"
echo "✓ Directory created/verified"

echo ""
echo "Copying NAPI binary:"
echo "  Source: $src"
echo "  Dest:   $dest"
cp -f "$src" "$dest"

echo "✓ Copy completed"
echo ""
echo "Result:"
ls -lh "$platform_npm_dir"
echo ""
echo "npm/$platform_dir directory contents:"
find "$platform_npm_dir" -type f

echo ""
echo "Including PDFium runtime..."
pdfium_src=""
for candidate in \
	"crates/kreuzberg-node/${pdfium_file}" \
	"target/release/${pdfium_file}" \
	"target/${target}/release/${pdfium_file}"; do
	if [ -f "$candidate" ]; then
		pdfium_src="$candidate"
		echo "  ✓ Found PDFium: $candidate"
		break
	fi
done

if [ -z "$pdfium_src" ]; then
	echo "  ⚠ Warning: ${pdfium_file} not found in any expected location" >&2
	echo "  Expected locations:" >&2
	echo "    - crates/kreuzberg-node/${pdfium_file}" >&2
	echo "    - target/release/${pdfium_file}" >&2
	echo "    - target/${target}/release/${pdfium_file}" >&2
else
	echo "  Copying ${pdfium_file} to platform directory..."
	cp -f "$pdfium_src" "crates/kreuzberg-node/npm/${platform_dir}/${pdfium_file}"
	ls -lh "crates/kreuzberg-node/npm/${platform_dir}/${pdfium_file}"

	platform_pkg_json="crates/kreuzberg-node/npm/${platform_dir}/package.json"
	tmp_pkg_json="$(mktemp)"
	trap 'rm -f "$tmp_pkg_json"' EXIT
	jq --arg f "$pdfium_file" '.files |= ((. + [$f]) | unique)' "$platform_pkg_json" >"$tmp_pkg_json"
	mv "$tmp_pkg_json" "$platform_pkg_json"
	echo "  ✓ Updated package.json to include ${pdfium_file}"
	echo ""
	echo "Updated package.json files:"
	cat "$platform_pkg_json"
fi

platform_npm_dir="crates/kreuzberg-node/npm/${platform_dir}"
if [ ! -d "$platform_npm_dir" ]; then
	echo "ERROR: Platform npm directory missing: $platform_npm_dir" >&2
	exit 1
fi

echo "Creating tarball with platform directory: $platform_dir"
tar -czf "node-bindings-${target}.tar.gz" -C crates/kreuzberg-node/npm "${platform_dir}"
