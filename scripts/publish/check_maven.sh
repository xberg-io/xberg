#!/usr/bin/env bash
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
	echo "Usage: $0 <version>" >&2
	exit 1
fi

version="$1"
group="dev.kreuzberg"
artifact="kreuzberg"
group_path="${group//.//}"
repo_url="https://repo1.maven.org/maven2/${group_path}/${artifact}/${version}/${artifact}-${version}.jar"
max_attempts=12
attempt=1
found=false

while [ $attempt -le $max_attempts ]; do
	echo "::debug::Checking Maven Central repository for ${group}:${artifact}:${version} (attempt ${attempt}/${max_attempts})" >&2

	if curl \
		--silent \
		--show-error \
		--retry 2 \
		--retry-delay 3 \
		--connect-timeout 30 \
		--max-time 30 \
		-I \
		"$repo_url" 2>/dev/null | grep -q "^HTTP.*200\|^HTTP.*301\|^HTTP.*302"; then
		found=true
		echo "::notice::Found ${group}:${artifact}:${version} in Maven Central repository after ${attempt} attempt(s)" >&2
		break
	fi

	if [ $attempt -lt $max_attempts ]; then
		sleep_time=$((attempt * 5))
		echo "::warning::Package not found yet, retrying in ${sleep_time}s... (attempt ${attempt}/${max_attempts})" >&2
		sleep "$sleep_time"
	fi

	attempt=$((attempt + 1))
done

if [ "$found" = true ]; then
	echo "exists=true"
	echo "::notice::Java package ${group}:${artifact}:${version} already exists on Maven Central" >&2
else
	echo "exists=false"
	echo "::notice::Java package ${group}:${artifact}:${version} not found on Maven Central, will build and publish" >&2
fi
