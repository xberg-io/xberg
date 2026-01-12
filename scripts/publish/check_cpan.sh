#!/bin/bash
# Check if a specific version of Kreuzberg exists on CPAN
# Usage: ./check_cpan.sh VERSION
# Output: exists=true or exists=false

set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 VERSION" >&2
    exit 1
fi

# MetaCPAN API endpoint
API_URL="https://fastapi.metacpan.org/v1/release/Kreuzberg"

# Try to get the release info
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL")

if [[ "$HTTP_STATUS" == "200" ]]; then
    # Get the version from the API
    CPAN_VERSION=$(curl -s "$API_URL" | python3 -c "import sys, json; print(json.load(sys.stdin).get('version', ''))" 2>/dev/null || echo "")

    if [[ "$CPAN_VERSION" == "$VERSION" ]]; then
        echo "exists=true"
        echo "::notice::Kreuzberg $VERSION already exists on CPAN"
    else
        echo "exists=false"
        echo "::notice::Kreuzberg $VERSION not found on CPAN (latest: $CPAN_VERSION)"
    fi
else
    # Package not found or error
    echo "exists=false"
    echo "::notice::Kreuzberg not found on CPAN or API error (HTTP $HTTP_STATUS)"
fi
