#!/usr/bin/env bash
#
# Retry + timeout helpers for CI scripts.
#
# Usage:
#   source "$REPO_ROOT/scripts/lib/retry.sh"
#   retry_with_backoff cmd args...
#   run_with_timeout 30 cmd args...
#   retry_with_backoff_timeout 300 cmd args...
#

set -euo pipefail

# Run a command with a timeout.
# - Returns 124 on timeout (GNU timeout convention).
run_with_timeout() {
	local seconds="$1"
	shift

	if command -v timeout >/dev/null 2>&1; then
		timeout "${seconds}" "$@"
		return $?
	fi
	if command -v gtimeout >/dev/null 2>&1; then
		gtimeout "${seconds}" "$@"
		return $?
	fi

	if command -v python3 >/dev/null 2>&1; then
		python3 - "$seconds" "$@" <<'PY'
import subprocess
import sys

timeout_s = int(sys.argv[1])
cmd = sys.argv[2:]
try:
    completed = subprocess.run(cmd, timeout=timeout_s)
    sys.exit(completed.returncode)
except subprocess.TimeoutExpired:
    sys.exit(124)
PY
		return $?
	fi

	"$@"
}

# Retry a command with exponential backoff.
retry_with_backoff() {
	local max_attempts=3
	local attempt=1
	local delay=5

	while [ $attempt -le $max_attempts ]; do
		if "$@"; then
			return 0
		fi

		if [ $attempt -lt $max_attempts ]; then
			echo "⚠ Attempt $attempt failed, retrying in ${delay}s..." >&2
			sleep $delay
			delay=$((delay * 2))
		fi
		attempt=$((attempt + 1))
	done

	return 1
}

# Retry a command with exponential backoff, enforcing a timeout per attempt.
# - Returns 124 if the final attempt timed out.
retry_with_backoff_timeout() {
	local seconds="$1"
	shift
	local max_attempts=3
	local attempt=1
	local delay=5
	local exit_code=1

	while [ $attempt -le $max_attempts ]; do
		if run_with_timeout "$seconds" "$@"; then
			return 0
		fi

		exit_code=$?
		if [ $attempt -lt $max_attempts ]; then
			echo "⚠ Attempt $attempt failed (exit $exit_code), retrying in ${delay}s..." >&2
			sleep $delay
			delay=$((delay * 2))
		fi
		attempt=$((attempt + 1))
	done

	return $exit_code
}

export -f run_with_timeout
export -f retry_with_backoff
export -f retry_with_backoff_timeout
