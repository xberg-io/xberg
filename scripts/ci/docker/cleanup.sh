#!/usr/bin/env bash

set -euo pipefail

VARIANT="${1:-}"

if [ -z "$VARIANT" ]; then
	echo "Usage: cleanup.sh <variant>"
	echo "  variant: core or full"
	exit 1
fi

echo "=== Cleaning up Docker resources ==="

docker ps -aq --filter "name=kreuzberg-test" | xargs -r docker rm -f || true

docker rmi "kreuzberg:$VARIANT" || true

docker system prune -af --volumes || true

echo "=== Final disk space ==="
df -h /
