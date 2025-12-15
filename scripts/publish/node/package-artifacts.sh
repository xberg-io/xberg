#!/usr/bin/env bash

set -euo pipefail

target="${TARGET:?TARGET not set}"

mkdir -p crates/kreuzberg-node/artifacts
pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts
if [ ! -d crates/kreuzberg-node/npm ]; then
	echo "npm artifact directory missing" >&2
	exit 1
fi

case "$target" in
aarch64-apple-darwin)
	platform_dir="darwin-arm64"
	node_file="kreuzberg-node.darwin-arm64.node"
	;;
x86_64-apple-darwin)
	platform_dir="darwin-x64"
	node_file="kreuzberg-node.darwin-x64.node"
	;;
x86_64-pc-windows-msvc)
	platform_dir="win32-x64-msvc"
	node_file="kreuzberg-node.win32-x64-msvc.node"
	;;
aarch64-pc-windows-msvc)
	platform_dir="win32-arm64-msvc"
	node_file="kreuzberg-node.win32-arm64-msvc.node"
	;;
x86_64-unknown-linux-gnu)
	platform_dir="linux-x64-gnu"
	node_file="kreuzberg-node.linux-x64-gnu.node"
	;;
aarch64-unknown-linux-gnu)
	platform_dir="linux-arm64-gnu"
	node_file="kreuzberg-node.linux-arm64-gnu.node"
	;;
armv7-unknown-linux-gnueabihf)
	platform_dir="linux-arm-gnueabihf"
	node_file="kreuzberg-node.linux-arm-gnueabihf.node"
	;;
*)
	echo "Unsupported NAPI target: $target" >&2
	exit 1
	;;
esac

dest="crates/kreuzberg-node/npm/${platform_dir}/${node_file}"
src=""
for candidate in "crates/kreuzberg-node/artifacts/${node_file}" "crates/kreuzberg-node/${node_file}"; do
	if [ -f "$candidate" ]; then
		src="$candidate"
		break
	fi
done

if [ -z "$src" ]; then
	echo "Missing built NAPI binary: expected ${node_file} under crates/kreuzberg-node/artifacts or crate root" >&2
	find crates/kreuzberg-node -maxdepth 2 -type f -name "*.node" -print || true
	exit 1
fi

mkdir -p "$(dirname "$dest")"
cp -f "$src" "$dest"
ls -la "$(dirname "$dest")"

tar -czf "node-bindings-${target}.tar.gz" -C crates/kreuzberg-node npm
