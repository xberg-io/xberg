#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

echo "=== Installing Ruby dependencies ==="
cd "$REPO_ROOT/packages/ruby"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  if [[ -z "${BUNDLE_GEMFILE:-}" ]]; then
    echo "BUNDLE_GEMFILE=$REPO_ROOT/packages/ruby/Gemfile" >> "$GITHUB_ENV"
  fi
  if [[ -z "${BUNDLE_PATH:-}" ]]; then
    echo "BUNDLE_PATH=$REPO_ROOT/packages/ruby/vendor/bundle" >> "$GITHUB_ENV"
  fi
fi

bundle config set deployment false
bundle config set path vendor/bundle
bundle install --jobs 4

echo "Ruby dependencies installed"
