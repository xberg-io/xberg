#!/usr/bin/env bash

# Check if Java package version exists on Maven Central
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"
group="dev.kreuzberg"
artifact="kreuzberg"

group_path="${group//.//}"
repo_url="https://repo1.maven.org/maven2/${group_path}/${artifact}/${version}/${artifact}-${version}.jar"

if curl -sI "$repo_url" 2>/dev/null | grep -q "^HTTP.*200\|^HTTP.*301\|^HTTP.*302"; then
  echo "exists=true"
  echo "::notice::Java package ${group}:${artifact}:${version} already exists on Maven Central"
else
  echo "exists=false"
  echo "::notice::Java package ${group}:${artifact}:${version} not found on Maven Central, will build and publish"
fi
