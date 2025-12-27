#!/usr/bin/env bash

set -euo pipefail

artifacts_dir="${1:?Artifacts directory argument required}"

if [ ! -d "$artifacts_dir" ]; then
	echo "Error: Artifacts directory not found: $artifacts_dir" >&2
	exit 1
fi

echo "Extracting bottle hashes from: $artifacts_dir"

for bottle in "$artifacts_dir"/kreuzberg--*.bottle.tar.gz; do
	if [ -f "$bottle" ]; then
		filename="$(basename "$bottle")"

		without_suffix="${filename%.bottle.tar.gz}"
		bottle_tag="${without_suffix##*.}"

		sha256=$(shasum -a 256 "$bottle" | cut -d' ' -f1)

		echo "${bottle_tag}=${sha256}" >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"

		echo "  $bottle_tag: $sha256"
	fi
done

echo "Bottle hashes extracted"
