#!/usr/bin/env bash
set -euo pipefail

# Update Formula/kreuzberg.rb in the homebrew-tap with the new tag's URL and
# source-tarball SHA256. The bottle DSL is updated separately by the
# `homebrew-merge-bottles@v1` action after bottles are built.
#
# Usage (env vars):
#   TAG=v5.0.0-rc.2 VERSION=5.0.0-rc.2 \
#   TAP_DIR=/path/to/homebrew-tap \
#   ./update-homebrew-formula.sh

tag="${TAG:?TAG is required (e.g. v5.0.0-rc.2)}"
version="${VERSION:?VERSION is required (e.g. 5.0.0-rc.2)}"
tap_dir="${TAP_DIR:?TAP_DIR is required (path to homebrew-tap checkout)}"
dry_run="${DRY_RUN:-false}"

formula="${tap_dir}/Formula/kreuzberg.rb"

[[ -f "$formula" ]] || {
  echo "Missing $formula" >&2
  exit 1
}

tarball_url="https://github.com/kreuzberg-dev/kreuzberg/archive/${tag}.tar.gz"

echo "Updating Homebrew formula for kreuzberg ${version} (tag ${tag})"

if [[ "$dry_run" == "true" ]]; then
  echo "[dry-run] target formula: $formula"
  echo "[dry-run] would set url to: $tarball_url"
  echo "[dry-run] would compute sha256 of source tarball and rewrite the formula"
  echo "[dry-run] would leave bottle DSL untouched (handled by homebrew-merge-bottles)"
  exit 0
fi

echo "Fetching source tarball SHA256 for ${tag}..."
sha256=$(curl -fsSL "$tarball_url" | shasum -a 256 | awk '{print $1}')
echo "  url:    $tarball_url"
echo "  sha256: $sha256"

# Update the top-level url + sha256 lines (the ones outside `bottle do ... end`).
# Match `url "..."` on one line, `sha256 "..."` on the next, only when both come
# before the `bottle do` block.
python3 - "$formula" "$tarball_url" "$sha256" <<'PY'
import re
import sys

formula_path, new_url, new_sha = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(formula_path).read()

# Split off the bottle block so the regex only touches the formula header.
bottle_start = text.find("bottle do")
if bottle_start == -1:
    head, tail = text, ""
else:
    head, tail = text[:bottle_start], text[bottle_start:]

head = re.sub(r'^(\s*url\s+)"[^"]*"', rf'\1"{new_url}"', head, count=1, flags=re.MULTILINE)
head = re.sub(r'^(\s*sha256\s+)"[^"]*"', rf'\1"{new_sha}"', head, count=1, flags=re.MULTILINE)

with open(formula_path, "w") as f:
    f.write(head + tail)
PY

echo "Updated $formula"
