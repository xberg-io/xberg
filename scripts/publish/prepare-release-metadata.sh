#!/usr/bin/env bash

set -euo pipefail

metadata="$(bash scripts/publish/validate-and-compute-release-metadata.sh)"
echo "$metadata" >release-metadata.json

outputs="$(printf '%s\n' "$metadata" | jq -r 'to_entries[] | "\(.key)=\(.value)"')"
printf '%s\n' "$outputs" >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"
