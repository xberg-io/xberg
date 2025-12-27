#!/usr/bin/env bash
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
	echo "Usage: $0 <version>" >&2
	exit 1
fi

VERSION="$1"
DRY_RUN="${DRY_RUN:-false}"
PACKAGE_NAME="kreuzberg/kreuzberg"

echo "::group::Publishing to Packagist"
echo "Package: ${PACKAGE_NAME}"
echo "Version: ${VERSION}"
echo "Dry run: ${DRY_RUN}"

echo "::notice::Packagist updates automatically via GitHub webhook"
echo "::notice::Waiting for Packagist to detect the new tag..."

if [[ "$DRY_RUN" == "true" ]]; then
	echo "::notice::Dry run mode - skipping Packagist verification"
	echo "::endgroup::"
	exit 0
fi

echo "Waiting 30 seconds for webhook processing..."
sleep 30

MAX_ATTEMPTS=12
ATTEMPT=1

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
	echo "Checking Packagist (attempt ${ATTEMPT}/${MAX_ATTEMPTS})..."

	RESPONSE=$(curl \
		--silent \
		--show-error \
		--retry 3 \
		--retry-delay 5 \
		"https://repo.packagist.org/p2/${PACKAGE_NAME}.json" 2>/dev/null || echo "{}")

	if echo "$RESPONSE" | jq -e ".packages[\"${PACKAGE_NAME}\"] | any(.version == \"${VERSION}\")" >/dev/null 2>&1; then
		echo "::notice::✓ Package ${PACKAGE_NAME}:${VERSION} is now available on Packagist"
		echo "::notice::View at: https://packagist.org/packages/${PACKAGE_NAME}#${VERSION}"
		echo "::endgroup::"
		exit 0
	fi

	if [ $ATTEMPT -lt $MAX_ATTEMPTS ]; then
		echo "Version not found yet, waiting 10 seconds..."
		sleep 10
	fi

	ATTEMPT=$((ATTEMPT + 1))
done

echo "::warning::Package version not found on Packagist after ${MAX_ATTEMPTS} attempts"
echo "::warning::This may be a timing issue. Check Packagist manually:"
echo "::warning::  https://packagist.org/packages/${PACKAGE_NAME}"
echo "::warning::The package should appear once the GitHub webhook is processed."

echo "::endgroup::"
exit 0
