#!/usr/bin/env bash

set -euo pipefail

crate="${1:?Crate name argument required}"
wait_seconds="${2:-0}"

if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
	echo "::error::CARGO_REGISTRY_TOKEN secret not set"
	exit 1
fi

if [ "$wait_seconds" -gt 0 ]; then
	echo "Waiting $wait_seconds seconds before publishing $crate..."
	sleep "$wait_seconds"
fi

publish_log=$(mktemp)
set +e
cargo publish -p "$crate" --token "$CARGO_REGISTRY_TOKEN" 2>&1 | tee "$publish_log"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -ne 0 ]; then
	if grep -qiE "(already uploaded|already exists)" "$publish_log"; then
		echo "::notice::$crate already published; skipping."
	else
		rm -f "$publish_log"
		exit "$status"
	fi
fi

rm -f "$publish_log"
echo "$crate published to crates.io"
