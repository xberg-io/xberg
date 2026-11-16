#!/usr/bin/env bash
set -euo pipefail

tag="${1:?Release tag argument required (e.g. v4.0.0-rc.7)}"

version="${tag#v}"
major="${version%%.*}"

module_tag="packages/go/v${major}/${tag}"
legacy_tag="packages/go/${tag}"

repo="${GITHUB_REPOSITORY:-xberg-io/xberg}"
sha=$(git rev-parse "$tag^{commit}")

create_tag() {
  local t="$1"

  local existing=""

  # A re-run must be a no-op when the tag already names this release's commit, and must
  # fail rather than leave a module tag on a different commit than the release. ~keep
  git fetch --quiet --force origin "refs/tags/${t}:refs/tags/${t}" 2>/dev/null || true
  existing=$(git rev-parse --verify --quiet "${t}^{commit}" || true)
  if [[ -n "$existing" ]]; then
    if [[ "$existing" == "$sha" ]]; then
      echo "::notice::Go module tag $t already points at ${sha:0:12}; skipping."
      return
    fi
    echo "::error::Go module tag $t points at ${existing:0:12}, not the release commit ${sha:0:12}" >&2
    exit 1
  fi

  git tag -a "$t" "$tag" -m "Go module tag ${t}"

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
