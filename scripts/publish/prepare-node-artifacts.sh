#!/usr/bin/env bash

set -euo pipefail

artifacts_dir="${1:-node-artifacts}"
typescript_defs_dir="${2:-typescript-defs}"
dest_dir="${3:-crates/kreuzberg-node}"

if [ ! -d "$artifacts_dir" ]; then
	echo "Error: Artifacts directory not found: $artifacts_dir" >&2
	exit 1
fi

rm -rf "$dest_dir/npm"
mkdir -p "$dest_dir/npm"

shopt -s nullglob
for pkg in "$artifacts_dir"/*.tar.gz; do
	echo "Unpacking $pkg"
	tmpdir=$(mktemp -d)
	tar -xzf "$pkg" -C "$tmpdir"

	echo "Contents of $tmpdir:"
	find "$tmpdir" -maxdepth 2 -type d

	while IFS= read -r -d '' platform_dir; do
		dir_name=$(basename "$platform_dir")
		echo "Processing platform directory: $dir_name"

		dest="$dest_dir/npm/$dir_name"
		echo "  Destination: $dest"

		if [ -z "$(find "$platform_dir" -maxdepth 1 -type f -print -quit)" ]; then
			echo "  ⚠ Warning: $dir_name appears to be empty, skipping"
			continue
		fi

		rm -rf "$dest"
		cp -R "$platform_dir" "$dest"

		if [ -d "$dest" ]; then
			file_count=$(find "$dest" -type f | wc -l)
			echo "  ✓ Copied successfully ($file_count files)"
		else
			echo "  ✗ ERROR: Copy failed!"
		fi
	done < <(find "$tmpdir" -mindepth 1 -maxdepth 1 -type d -print0)

	rm -rf "$tmpdir"
done

echo ""
echo "=== Final npm directory structure ==="
find "$dest_dir/npm" -type f | sort

if [ -d "$typescript_defs_dir" ]; then
	cp "$typescript_defs_dir"/index.js "$typescript_defs_dir"/index.d.ts "$dest_dir/" || true
	echo "TypeScript definitions merged"
fi

echo "Node artifacts prepared successfully"
