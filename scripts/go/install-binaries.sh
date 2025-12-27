#!/usr/bin/env bash

set -euo pipefail

# shellcheck disable=SC2034
RED='\033[0;31m'
# shellcheck disable=SC2034
GREEN='\033[0;32m'
# shellcheck disable=SC2034
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'
# shellcheck disable=SC2034
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

tag=""
dest="${KREUZBERG_INSTALL_DEST:-}"
skip_build="${KREUZBERG_SKIP_BUILD:-false}"
verbose="false"

while [[ $# -gt 0 ]]; do
	case $1 in
	-t | --tag)
		tag="$2"
		shift 2
		;;
	-d | --dest)
		dest="$2"
		shift 2
		;;
	--skip-build-fallback)
		skip_build="true"
		shift
		;;
	-v | --verbose)
		verbose="true"
		shift
		;;
	-h | --help)
		head -28 "$0" | tail -n +2
		exit 0
		;;
	*)
		echo "Unknown option: $1" >&2
		exit 1
		;;
	esac
done

go_args=("-v" "scripts/go/download-binaries.go")
if [[ -n "$tag" ]]; then
	go_args+=("-tag" "$tag")
fi
if [[ -n "$dest" ]]; then
	go_args+=("-dest" "$dest")
fi
if [[ "$skip_build" == "true" ]]; then
	go_args+=("-skip-build-fallback")
fi
if [[ "$verbose" == "true" ]]; then
	go_args+=("-verbose")
fi

if [[ "$verbose" == "true" ]]; then
	echo -e "${BLUE}Running: go run ${go_args[*]}${NC}"
fi

exec go run "${go_args[@]}"
