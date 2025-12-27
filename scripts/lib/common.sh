#!/usr/bin/env bash

set -euo pipefail

get_repo_root() {
	local start_dir current_dir
	start_dir="$(pwd)"
	current_dir="$start_dir"

	while [ "$current_dir" != "/" ]; do
		if [ -f "$current_dir/Cargo.toml" ]; then
			echo "$current_dir"
			return 0
		fi
		current_dir="$(dirname "$current_dir")"
	done

	echo "Error: Could not find repository root (Cargo.toml) from: $start_dir" >&2
	return 1
}

validate_repo_root() {
	local repo_root="${1:-${REPO_ROOT:-}}"

	if [ -z "$repo_root" ]; then
		echo "Error: REPO_ROOT not provided and env var not set" >&2
		return 1
	fi

	if [ ! -f "$repo_root/Cargo.toml" ]; then
		echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $repo_root/Cargo.toml" >&2
		echo "REPO_ROOT resolved to: $repo_root" >&2
		return 1
	fi

	return 0
}

error_exit() {
	local message="${1:-Unknown error}"
	local exit_code="${2:-1}"
	echo "Error: $message" >&2
	exit "$exit_code"
}

get_platform() {
	if [ -n "${RUNNER_OS:-}" ]; then
		echo "$RUNNER_OS"
	else
		case "$(uname -s)" in
		Linux*)
			echo "Linux"
			;;
		Darwin*)
			echo "macOS"
			;;
		MINGW* | MSYS* | CYGWIN*)
			echo "Windows"
			;;
		*)
			echo "unknown"
			;;
		esac
	fi
}

export -f get_repo_root
export -f validate_repo_root
export -f error_exit
export -f get_platform
