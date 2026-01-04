#!/usr/bin/env bash
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"
package="@kreuzberg/wasm"
max_attempts=3
attempt=1
package_found=false

while [ $attempt -le $max_attempts ]; do
  echo "::debug::Checking npm for ${package}@${version} (attempt ${attempt}/${max_attempts})" >&2

  if npm view "${package}@${version}" version >/dev/null 2>&1; then
    package_found=true
    break
  elif [ $attempt -lt $max_attempts ]; then
    sleep_time=$((attempt * 5))
    echo "::warning::npm check failed, retrying in ${sleep_time}s..." >&2
    sleep "$sleep_time"
  fi

  attempt=$((attempt + 1))
done

if [ "$package_found" = true ]; then
  echo "exists=true"
  echo "::notice::WASM package ${package}@${version} already exists on npm" >&2
else
  echo "exists=false"
  echo "::notice::WASM package ${package}@${version} not found on npm, will build and publish" >&2
fi
