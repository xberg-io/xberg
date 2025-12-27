#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:?TARGET environment variable must be set}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ARTIFACT_DIR="$ROOT/crates/kreuzberg-node/artifacts"
if [[ ! -d "$ARTIFACT_DIR" ]]; then
	echo "ERROR: Artifacts directory not found at $ARTIFACT_DIR" >&2
	echo "You must build the NAPI module first (e.g., task typescript:build)" >&2
	exit 1
fi

shopt -s nullglob
NODE_FILES=("$ARTIFACT_DIR"/*.node)
if [[ ${#NODE_FILES[@]} -eq 0 ]]; then
	echo "ERROR: No .node files found in $ARTIFACT_DIR" >&2
	echo "The native module must be built first (e.g., task typescript:build)" >&2
	exit 1
fi

echo "Found ${#NODE_FILES[@]} .node file(s) in $ARTIFACT_DIR"

pushd "$ROOT" >/dev/null

pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts

if [[ ! -d crates/kreuzberg-node/npm ]]; then
	echo "ERROR: npm artifact directory missing after running napi artifacts" >&2
	exit 1
fi

tar -czf "node-bindings-${TARGET}.tar.gz" -C crates/kreuzberg-node npm
echo "Created node-bindings-${TARGET}.tar.gz"

popd >/dev/null
