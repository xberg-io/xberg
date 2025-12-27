#!/usr/bin/env bash

set -euo pipefail

tag="${1:?Release tag argument required}"
artifacts_dir="${2:-dist/csharp}"

if [ ! -d "$artifacts_dir" ]; then
	echo "Error: Artifacts directory not found: $artifacts_dir" >&2
	exit 1
fi

for file in "$artifacts_dir"/*.nupkg; do
	if [ -f "$file" ]; then
		gh release upload "$tag" "$file" --clobber
		echo "Uploaded $(basename "$file")"
	fi
done

echo "C# NuGet packages uploaded to $tag"
