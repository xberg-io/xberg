#!/usr/bin/env bash

set -euo pipefail

tag="${1:?Release tag argument required}"

if ! gh release view "$tag" >/dev/null 2>&1; then
  gh release create "$tag" --title "$tag" --generate-notes --draft=false
  echo "Created published release $tag"
else
  # Ensure existing release is published (not draft)
  current_draft=$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null || echo "false")
  if [ "$current_draft" = "true" ]; then
    echo "Release exists but is in draft state. Publishing..."
    gh release edit "$tag" --draft=false
    echo "Published release $tag"
  fi
fi
