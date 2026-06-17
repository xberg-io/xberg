#!/usr/bin/env bash
set -euo pipefail

tag="${1:?Release tag argument required (e.g. v4.0.0-rc.7)}"

version="${tag#v}"
major="${version%%.*}"

# Two tag formats for backwards compatibility:
#   - packages/go/v4/vX.Y.Z  (correct per Go module spec, module path includes /v4)
#   - packages/go/vX.Y.Z     (legacy format, existing consumers may depend on it)
module_tag="packages/go/v${major}/${tag}"
legacy_tag="packages/go/${tag}"

repo="${GITHUB_REPOSITORY:-kreuzberg-dev/kreuzberg}"
sha=$(git rev-parse "$tag^{commit}")

create_tag() {
  local t="$1"

  if git rev-parse "$t" >/dev/null 2>&1; then
    echo "::notice::Go module tag $t already exists locally; skipping."
    return
  fi

  if git ls-remote --tags origin | grep -q "refs/tags/${t}$"; then
    echo "::notice::Go module tag $t already exists on remote; skipping."
    return
  fi

  git tag -a "$t" "$tag" -m "Go module tag ${t}"

  # Push the tag directly. The job has contents:write permission.
  # If GITHUB_TOKEN is blocked by tag protection rules, fall back to
  # the GitHub API (which may also fail, but gives a clearer error).
  if ! git push origin "refs/tags/${t}" 2>/dev/null; then
    echo "::warning::git push failed for tag $t, trying GitHub API..."
    gh api "repos/${repo}/git/refs" \
      -f "ref=refs/tags/${t}" \
      -f "sha=${sha}" \
      --silent
  fi

  echo "✅ Go module tag created: $t (sha: ${sha:0:12})"
}

create_tag "$module_tag"
create_tag "$legacy_tag"
