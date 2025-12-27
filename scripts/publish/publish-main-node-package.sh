#!/usr/bin/env bash

# Includes idempotent handling for already-published versions.
# CRITICAL: Respects npm dist-tag to prevent pre-release versions from being tagged 'latest'

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "${SCRIPT_DIR}/lib/common.sh"

pkg_dir="${1:-crates/kreuzberg-node}"
npm_tag="${2:-${NPM_TAG:-latest}}"

validate_directory "$pkg_dir" "Package directory"

if ! publish_npm_from_directory "$pkg_dir" "$npm_tag"; then
	exit 1
fi

log_success "@kreuzberg/node published to npm with tag '$npm_tag'"
