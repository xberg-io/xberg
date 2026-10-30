#!/usr/bin/env bash
set -euo pipefail

label="${1:-Node setup}"

echo "=== ${label} ==="
node_version="$(node --version)"
echo "Node version: $node_version"
pnpm_version="$(pnpm --version)"
echo "pnpm version: $pnpm_version"
echo "tsx availability: $(command -v tsx || echo 'NOT FOUND')"
echo "pnpm workspace structure:"
pnpm list --depth=0 || echo 'Warning: optional pnpm workspace listing failed' >&2
