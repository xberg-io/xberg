#!/usr/bin/env bash

# Convention: packages/go/v4/v<version>

set -euo pipefail

tag="${1:?Release tag argument required}"

version="${tag#v}"

module_prefix="packages/go/v4"
module_tag="${module_prefix}/v${version}"

echo "Creating Go module tag: $module_tag"

git tag "$module_tag" "$tag"

echo "✅ Go module tag created: $module_tag"
echo "   This enables: go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@v${version}"
