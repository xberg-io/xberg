#!/usr/bin/env bash
set -euo pipefail

echo "=== Running Docker CLI feature tests ==="
python3 -m unittest scripts/ci/docker/test_docker_unit.py scripts/ci/docker/test_verify_image_notices.py
python3 scripts/ci/docker/test_docker.py --image "xberg:cli" --variant cli --verbose
