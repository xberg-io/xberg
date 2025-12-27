#!/usr/bin/env bash
set -euo pipefail

pkg_dir="${1:-crates/kreuzberg-node}"

if [ ! -d "$pkg_dir" ]; then
	echo "Package directory not found: $pkg_dir" >&2
	exit 1
fi

npm_dir="$pkg_dir/npm"
if [ ! -d "$npm_dir" ]; then
	echo "Platform npm directory not found: $npm_dir" >&2
	exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
	echo "jq is required to stage optionalDependencies for the main Node package" >&2
	exit 1
fi

tmp_pkg_json="$(mktemp)"
trap 'rm -f "$tmp_pkg_json"' EXIT

optional_deps_json='{}'

shopt -s nullglob
for platform_pkg_json in "$npm_dir"/*/package.json; do
	name="$(jq -r '.name // empty' "$platform_pkg_json")"
	version="$(jq -r '.version // empty' "$platform_pkg_json")"

	if [ -z "$name" ] || [ -z "$version" ]; then
		echo "Invalid platform package.json: $platform_pkg_json" >&2
		exit 1
	fi

	optional_deps_json="$(jq -c --arg n "$name" --arg v "$version" '. + {($n): $v}' <<<"$optional_deps_json")"
done

if [ "$optional_deps_json" = "{}" ]; then
	echo "No platform packages found under $npm_dir" >&2
	exit 1
fi

jq --argjson deps "$optional_deps_json" '.optionalDependencies = $deps' "$pkg_dir/package.json" >"$tmp_pkg_json"
mv "$tmp_pkg_json" "$pkg_dir/package.json"
