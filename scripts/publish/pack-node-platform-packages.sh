#!/usr/bin/env bash

set -euo pipefail

npm_dir="${1:-crates/kreuzberg-node/npm}"

if [ ! -d "$npm_dir" ]; then
	echo "Error: npm directory not found: $npm_dir" >&2
	exit 1
fi

cd "$npm_dir"
for dir in */; do
	if [ -f "${dir}package.json" ]; then
		shopt -s nullglob
		node_bins=("${dir}"*.node)
		if [ "${#node_bins[@]}" -eq 0 ]; then
			echo "Error: missing .node binary in ${npm_dir}/${dir}" >&2
			ls -la "${npm_dir}/${dir}" || true
			exit 1
		fi
		(cd "$dir" && npm pack && mv ./*.tgz ..)
	fi
done
