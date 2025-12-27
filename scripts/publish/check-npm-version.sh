#!/usr/bin/env bash

# Check if Node package version exists on npm
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"
package="${2:-@kreuzberg/node}"

if npm view "${package}@${version}" version >/dev/null 2>&1; then
	echo "exists=true"
	echo "::notice::Node package ${package}@${version} already exists on npm" >&2
else
	echo "exists=false"
	echo "::notice::Node package ${package}@${version} not found on npm, will build and publish" >&2
fi
