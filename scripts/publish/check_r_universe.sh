#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: check_r_universe.sh <version>}"

RESPONSE=$(curl -sf "https://kreuzberg-dev.r-universe.dev/api/packages/kreuzberg" 2>/dev/null || echo '{}')
EXISTING=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('Version',''))" 2>/dev/null || echo "")

if [ "$EXISTING" = "$VERSION" ]; then
  echo "exists=true"
else
  echo "exists=false"
fi
