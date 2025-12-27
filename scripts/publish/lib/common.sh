#!/usr/bin/env bash

# - Robust publish logic with idempotent version detection

set -euo pipefail

SCRIPT_DIR="${SCRIPT_DIR:-.}"

readonly COLOR_RED='\033[0;31m'
readonly COLOR_GREEN='\033[0;32m'
readonly COLOR_YELLOW='\033[1;33m'
readonly COLOR_RESET='\033[0m'

declare -a CLEANUP_STACK=()

register_cleanup() {
	local handler="$1"
	CLEANUP_STACK+=("$handler")
}

_run_cleanups() {
	local i
	for ((i = ${#CLEANUP_STACK[@]} - 1; i >= 0; i--)); do
		eval "${CLEANUP_STACK[i]}" || true
	done
}

trap '_run_cleanups' EXIT

log_info() {
	local msg="$1"
	printf "[INFO] %s: %s\n" "$(date '+%H:%M:%S')" "$msg"
}

log_error() {
	local msg="$1"
	printf "${COLOR_RED}[ERROR] %s: %s${COLOR_RESET}\n" "$(date '+%H:%M:%S')" "$msg" >&2
}

log_warning() {
	local msg="$1"
	printf "${COLOR_YELLOW}[WARN] %s: %s${COLOR_RESET}\n" "$(date '+%H:%M:%S')" "$msg"
}

log_success() {
	local msg="$1"
	printf "${COLOR_GREEN}[OK] %s: %s${COLOR_RESET}\n" "$(date '+%H:%M:%S')" "$msg"
}

validate_directory() {
	local path="$1"
	local name="$2"

	if [ ! -d "$path" ]; then
		log_error "$name not found: $path"
		exit 1
	fi
}

validate_file() {
	local path="$1"
	local name="$2"

	if [ ! -f "$path" ]; then
		log_error "$name not found: $path"
		exit 1
	fi
}

is_already_published_npm() {
	local log_file="$1"

	if grep -qi "previously published" "$log_file" ||
		grep -qi "cannot publish over" "$log_file" ||
		grep -qi "cannot publish to repository" "$log_file"; then
		return 0
	fi
	return 1
}

publish_npm_package() {
	local pkg_path="$1"
	local npm_tag="${2:-latest}"
	local pkg_name

	if [ ! -f "$pkg_path" ]; then
		log_error "Package file not found: $pkg_path"
		return 1
	fi

	pkg_name="$(basename "$pkg_path")"
	log_info "Publishing $pkg_name with tag '$npm_tag'"

	local publish_log
	publish_log=$(mktemp) || {
		log_error "Failed to create temporary log file"
		return 1
	}
	register_cleanup "rm -f '$publish_log'"

	local status
	set +e
	project_npmrc=""
	if [ -f ".npmrc" ] && grep -Eq '^(shared-workspace-lockfile|auto-install-peers|hoist)=' ".npmrc"; then
		project_npmrc="$(mktemp)"
		mv -f ".npmrc" "$project_npmrc"
		register_cleanup "if [ -f '$project_npmrc' ]; then mv -f '$project_npmrc' .npmrc; fi"
	fi

	npm publish "$pkg_path" --access public --provenance --ignore-scripts --tag "$npm_tag" 2>&1 | tee "$publish_log"
	status=${PIPESTATUS[0]}
	set -e

	if [ "$status" -eq 0 ]; then
		log_success "$pkg_name published to npm with tag '$npm_tag'"
		return 0
	fi

	if is_already_published_npm "$publish_log"; then
		log_warning "$pkg_name already published; skipping"
		return 0
	fi

	log_error "Failed to publish $pkg_name"
	return 1
}

publish_npm_from_directory() {
	local pkg_dir="$1"
	local npm_tag="${2:-latest}"
	local pkg_name

	validate_directory "$pkg_dir" "Package directory"

	pkg_name="$(basename "$pkg_dir")"
	log_info "Publishing from $pkg_name with tag '$npm_tag'"

	local publish_log
	publish_log=$(mktemp) || {
		log_error "Failed to create temporary log file"
		return 1
	}
	register_cleanup "rm -f '$publish_log'"

	local status
	set +e
	(
		cd "$pkg_dir" || exit 1
		project_npmrc=""
		if [ -f ".npmrc" ] && grep -Eq '^(shared-workspace-lockfile|auto-install-peers|hoist)=' ".npmrc"; then
			project_npmrc="$(mktemp)"
			mv -f ".npmrc" "$project_npmrc"
			trap 'if [ -f "$project_npmrc" ]; then mv -f "$project_npmrc" .npmrc; fi' EXIT
		fi
		npm publish --access public --provenance --ignore-scripts --tag "$npm_tag" 2>&1 | tee "$publish_log"
	)
	status=${PIPESTATUS[0]}
	set -e

	if [ "$status" -eq 0 ]; then
		log_success "$pkg_name published to npm with tag '$npm_tag'"
		return 0
	fi

	if is_already_published_npm "$publish_log"; then
		log_warning "$pkg_name already published; skipping"
		return 0
	fi

	log_error "Failed to publish $pkg_name"
	return 1
}
