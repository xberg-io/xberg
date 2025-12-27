#!/usr/bin/env bash
# Check if PHP package version exists on Packagist
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
	echo "Usage: $0 <version>" >&2
	exit 1
fi

version="$1"
url="https://repo.packagist.org/p2/kreuzberg/kreuzberg.json"
max_attempts=3
attempt=1
exists="false"

while [ $attempt -le $max_attempts ]; do
	echo "::debug::Checking Packagist for kreuzberg/kreuzberg:${version} (attempt ${attempt}/${max_attempts})"

	response=$(curl \
		--silent \
		--show-error \
		--retry 3 \
		--retry-delay 5 \
		--connect-timeout 30 \
		--max-time 60 \
		"$url" 2>/dev/null || echo "{}")

	if echo "$response" | jq -e ".packages[\"kreuzberg/kreuzberg\"] | any(.version == \"${version}\")" >/dev/null 2>&1; then
		exists="true"
		break
	elif echo "$response" | jq -e '.packages' >/dev/null 2>&1; then
		exists="false"
		break
	fi

	if [ $attempt -lt $max_attempts ]; then
		sleep_time=$((attempt * 5))
		echo "::warning::Packagist check failed, retrying in ${sleep_time}s..."
		sleep "$sleep_time"
	fi

	attempt=$((attempt + 1))
done

if [ "$exists" = "true" ]; then
	echo "exists=true" >>"$GITHUB_OUTPUT"
	echo "::notice::PHP package kreuzberg/kreuzberg:${version} already exists on Packagist"
elif [ "$exists" = "false" ]; then
	echo "exists=false" >>"$GITHUB_OUTPUT"
	echo "::notice::PHP package kreuzberg/kreuzberg:${version} not found on Packagist (will auto-update via Git webhook)"
else
	echo "::error::Failed to check Packagist after $max_attempts attempts"
	exit 1
fi
