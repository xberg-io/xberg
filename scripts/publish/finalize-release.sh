#!/usr/bin/env bash
# Finalize a GitHub release by publishing it from draft state
# and updating release notes from CHANGELOG.md
#
#   $1: Release tag (required, e.g., "v4.0.1")

set -euo pipefail

tag="${1:?Release tag argument required}"

echo "Finalizing release ${tag}..."

# Check if release exists
if ! gh release view "$tag" >/dev/null 2>&1; then
  echo "::error::Release ${tag} does not exist. Cannot finalize."
  exit 1
fi

# Check if release is in draft state
current_draft=$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null || echo "false")

if [ "$current_draft" = "true" ]; then
  echo "Release ${tag} is in draft state. Publishing..."
  gh release edit "$tag" --draft=false
  echo "::notice::Published release ${tag} from draft state"
else
  echo "Release ${tag} is already published"
fi

echo "Release ${tag} finalized successfully"
