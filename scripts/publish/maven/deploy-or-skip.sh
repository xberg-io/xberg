#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat >&2 <<'EOF'
Usage:
  deploy-or-skip.sh --version <version> -- mvn <args...>
  deploy-or-skip.sh --check-log <path> --version <version>

Behavior:
  - Runs the Maven deploy command.
  - If deploy fails because the Maven Central component already exists,
    exits 0 (idempotent publish).
EOF
}

already_exists() {
	local log_file="$1"
	local version="$2"

	grep -Eq "Component with package url: 'pkg:maven/dev\\.kreuzberg/kreuzberg@${version}'.*already exists" "$log_file" ||
		grep -Eq "pkg:maven/dev\\.kreuzberg/kreuzberg@${version}.*already exists" "$log_file"
}

version=""
log_only=""

while [[ $# -gt 0 ]]; do
	case "$1" in
	--version)
		version="${2:-}"
		shift 2
		;;
	--check-log)
		log_only="${2:-}"
		shift 2
		;;
	--help | -h)
		usage
		exit 0
		;;
	--)
		shift
		break
		;;
	*)
		break
		;;
	esac
done

if [[ -z "$version" ]]; then
	echo "Missing required --version" >&2
	usage
	exit 2
fi

if [[ -n "$log_only" ]]; then
	if [[ ! -f "$log_only" ]]; then
		echo "Log file not found: $log_only" >&2
		exit 2
	fi
	if already_exists "$log_only" "$version"; then
		exit 0
	fi
	exit 1
fi

if [[ $# -lt 1 ]]; then
	echo "Missing Maven command after --" >&2
	usage
	exit 2
fi

log_file="$(mktemp)"
trap 'rm -f "$log_file"' EXIT

set +e
"$@" 2>&1 | tee "$log_file"
status=${PIPESTATUS[0]}
set -e

if [[ "$status" -eq 0 ]]; then
	exit 0
fi

if already_exists "$log_file" "$version"; then
	echo "::notice::Maven package dev.kreuzberg:kreuzberg:${version} already exists; skipping publish." >&2
	exit 0
fi

exit "$status"
