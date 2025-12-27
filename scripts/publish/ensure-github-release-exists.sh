#!/usr/bin/env bash

set -euo pipefail

tag="${1:?Release tag argument required}"

if ! gh release view "$tag" >/dev/null 2>&1; then
	gh release create "$tag" --title "$tag" --generate-notes
	echo "Created release $tag"
fi
