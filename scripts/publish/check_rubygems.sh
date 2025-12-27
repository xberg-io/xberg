#!/usr/bin/env bash
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
	echo "Usage: $0 <version>" >&2
	exit 1
fi

version="${1#v}"
package_name="kreuzberg"
url="https://rubygems.org/api/v1/versions/${package_name}.json"
max_attempts=3
attempt=1
http_code=""

normalize_rubygems_version() {
	local v="$1"
	if [[ "$v" == *-* ]]; then
		local base="${v%%-*}"
		local prerelease="${v#*-}"
		echo "${base}.pre.${prerelease//-/.}"
	else
		echo "$v"
	fi
}

rubygems_version="$(normalize_rubygems_version "$version")"
version_candidates=("$version")
if [[ "$rubygems_version" != "$version" ]]; then
	version_candidates+=("$rubygems_version")
fi

while [ $attempt -le $max_attempts ]; do
	echo "::debug::Checking RubyGems for ${package_name} versions: ${version_candidates[*]} (attempt ${attempt}/${max_attempts})" >&2

	http_code=$(curl \
		--silent \
		--show-error \
		--retry 3 \
		--retry-delay 5 \
		--connect-timeout 30 \
		--max-time 60 \
		-o /tmp/rubygems-check.json \
		-w "%{http_code}" \
		"$url" 2>/dev/null || echo "000")

	if [ "$http_code" = "200" ] || [ "$http_code" = "404" ]; then
		break
	fi

	if [ $attempt -lt $max_attempts ]; then
		sleep_time=$((attempt * 5))
		echo "::warning::RubyGems check failed (HTTP $http_code), retrying in ${sleep_time}s..." >&2
		sleep "$sleep_time"
	fi

	attempt=$((attempt + 1))
done

if [ "$http_code" = "200" ]; then
	found=false
	found_version=""
	for candidate in "${version_candidates[@]}"; do
		if jq -e ".[] | select(.number == \"${candidate}\")" /tmp/rubygems-check.json >/dev/null 2>&1; then
			found=true
			found_version="$candidate"
			break
		fi
	done

	if [ "$found" = "true" ]; then
		echo "exists=true"
		echo "::notice::Ruby gem ${package_name}==${found_version} already exists on RubyGems" >&2
	else
		echo "exists=false"
		echo "::notice::Ruby gem ${package_name} not found on RubyGems for versions: ${version_candidates[*]} (will build/publish)" >&2
	fi
elif [ "$http_code" = "404" ]; then
	echo "exists=false"
	echo "::notice::Ruby gem ${package_name} not found on RubyGems (first publish), will build and publish" >&2
else
	echo "::error::Failed to check RubyGems after $max_attempts attempts (last HTTP code: $http_code)"
	exit 1
fi
