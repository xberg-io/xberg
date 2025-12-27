#!/usr/bin/env bash

set -euo pipefail

tag="${TAG:?TAG not set}"
asset_prefix="${ASSET_PREFIX:?ASSET_PREFIX not set}"
summary_label="${SUMMARY_LABEL:-assets}"
required_assets="${REQUIRED_ASSETS:-}"

tmp_json="$(mktemp)"
exists=false

if gh release view "$tag" --json assets >"$tmp_json" 2>/dev/null; then
	if ! jq -e '.assets' "$tmp_json" >/dev/null 2>&1; then
		echo "::warning::Release '${tag}' found but has no assets" >&2
	else
		if [ -n "$required_assets" ]; then
			missing=0
			while IFS= read -r asset; do
				asset="$(echo "$asset" | xargs)"
				if [ -z "$asset" ]; then
					continue
				fi
				echo "::debug::Checking for required asset: ${asset}" >&2
				if ! jq -e --arg name "$asset" '.assets[].name | select(. == $name)' "$tmp_json" >/dev/null; then
					missing=1
					echo "::debug::Asset not found: ${asset}" >&2
					if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
						echo "${summary_label}: missing release asset '${asset}'" >>"$GITHUB_STEP_SUMMARY"
					fi
				else
					echo "::debug::Asset found: ${asset}" >&2
				fi
			done <<<"$required_assets"

			if [ "$missing" -eq 0 ]; then
				exists=true
			fi
		else
			if jq -e --arg prefix "$asset_prefix" '.assets[].name | select(startswith($prefix))' "$tmp_json" >/dev/null; then
				exists=true
			fi
		fi
	fi
else
	echo "::warning::Could not retrieve release '${tag}' from GitHub" >&2
fi

rm -f "$tmp_json"

echo "exists=$exists" >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"

if [ "$exists" = "true" ] && [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
	echo "${summary_label} artifacts already exist on release ${tag}; skipping." >>"$GITHUB_STEP_SUMMARY"
fi
