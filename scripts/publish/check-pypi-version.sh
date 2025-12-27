#!/usr/bin/env bash

# Check if Python package version exists on PyPI
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"

http_code=$(curl -s -o /dev/null -w "%{http_code}" \
	"https://pypi.org/pypi/kreuzberg/${version}/json")

if [ "$http_code" = "200" ]; then
	echo "exists=true"
	echo "::notice::Python package kreuzberg==${version} already exists on PyPI"
else
	echo "exists=false"
	echo "::notice::Python package kreuzberg==${version} not found on PyPI, will build and publish"
fi
