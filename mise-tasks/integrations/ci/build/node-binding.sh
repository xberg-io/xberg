#!/usr/bin/env bash
#MISE description="Build the local Node binding for integration tests"
set -euo pipefail

cd crates/xberg-node
pnpm install
CARGO_BUILD_JOBS=1 pnpm exec napi build --no-js --output-dir . --release --platform
mkdir -p ../../dist/node
cp index.js index.mjs binding.cjs index.d.ts package.json ./*.node ../../dist/node/
