#!/usr/bin/env bash

set -euo pipefail

PLATFORM="${PLATFORM:-}"

if [ -z "$PLATFORM" ]; then
	echo "::error::PLATFORM environment variable is required" >&2
	exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

LIB_DIR="$REPO_ROOT/target/release"

if [ ! -d "$LIB_DIR" ]; then
	echo "::error::Native library directory not found at $LIB_DIR" >&2
	exit 1
fi

setup_rust_ffi_paths "$REPO_ROOT"

echo "Ruby gem build environment:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LIB_DIR: $LIB_DIR"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-}"
echo ""

echo "Vendoring kreuzberg core..."
bash "$REPO_ROOT/scripts/ci/ruby/vendor-kreuzberg-core.sh"
echo ""

cd "$REPO_ROOT/packages/ruby"
echo "Building Ruby native gem in: $(pwd)"

echo "Installing Ruby dependencies..."
bundle install

bundle exec rake clean
bundle exec rake compile
bundle exec rake build

echo "Installing built gem..."
gem install pkg/kreuzberg-*.gem
