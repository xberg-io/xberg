#!/usr/bin/env bash

set -euo pipefail

LOG_DIR="${1:-/tmp/docker-logs}"

echo "=== Collecting Docker logs ==="
mkdir -p "$LOG_DIR"

for container in $(docker ps -a --filter "name=kreuzberg-test" --format "{{.Names}}"); do
	echo "Collecting logs from: $container"
	docker logs "$container" >"$LOG_DIR/${container}.log" 2>&1 || true
done

docker info >"$LOG_DIR/docker-info.txt" 2>&1 || true
docker version >"$LOG_DIR/docker-version.txt" 2>&1 || true

echo "=== Docker logs collected ==="
ls -lh "$LOG_DIR/"
