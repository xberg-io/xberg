#!/usr/bin/env bash

# Check if Ruby gem version exists on RubyGems
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"

if curl -s "https://rubygems.org/api/v1/versions/kreuzberg.json" | jq -e "any(.[]; .number == \"${version}\")" >/dev/null 2>&1; then
  echo "exists=true"
  echo "::notice::Ruby gem kreuzberg ${version} already exists on RubyGems"
else
  echo "exists=false"
  echo "::notice::Ruby gem kreuzberg ${version} not found on RubyGems, will build and publish"
fi
