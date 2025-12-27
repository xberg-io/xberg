#!/usr/bin/env bash

set -euo pipefail

echo "=== Running all lint checks in check-only mode ==="
task lint:check
echo "Lint checks complete"
