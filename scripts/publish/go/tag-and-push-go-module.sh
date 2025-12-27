#!/usr/bin/env bash
set -euo pipefail

tag="${1:?Release tag argument required (e.g. v4.0.0-rc.7)}"

version="${tag#v}"
module_tag="packages/go/v4/v${version}"

if git rev-parse "$module_tag" >/dev/null 2>&1; then
	echo "::notice::Go module tag $module_tag already exists; skipping."
	exit 0
fi

git tag "$module_tag" "$tag"
git push origin "$module_tag"

echo "✅ Go module tag created: $module_tag"
