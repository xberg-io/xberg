#!/usr/bin/env bash
set -euo pipefail

if [ -f "Cargo.lock" ]; then
	if command -v sha256sum &>/dev/null; then
		hash="$(sha256sum Cargo.lock | cut -d' ' -f1)"
		echo "Generated Cargo.lock hash using sha256sum"
	elif command -v shasum &>/dev/null; then
		hash="$(shasum -a 256 Cargo.lock | cut -d' ' -f1)"
		echo "Generated Cargo.lock hash using shasum"
	else
		hash="$(stat -c %Y Cargo.lock 2>/dev/null || stat -f %m Cargo.lock)"
		echo "WARNING: Neither sha256sum nor shasum available, using timestamp-based hash"
		echo "WARNING: This may cause unnecessary cache invalidation"
	fi
else
	hash="$(date +%Y%m%d)"
	echo "WARNING: Cargo.lock not found, using date-based hash"
fi

echo "hash=$hash" >>"$GITHUB_OUTPUT"
echo "Using Cargo.lock hash: $hash"
