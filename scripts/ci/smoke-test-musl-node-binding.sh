#!/usr/bin/env bash
# Installs a built musl Node native binding (.node file) into a genuine
# Alpine (musl libc) container and runs a real extraction against a known
# fixture, asserting the exact expected text comes back. See
# scripts/ci/smoke_test_node_binding.js for why a require()-only check is not
# sufficient here (same rationale as xberg-io/xberg#490 for the Python wheel).
#
# Deliberately does NOT run on the GitHub Actions host directly: the host is
# glibc (ubuntu-latest / ubuntu-24.04-arm), and the entire point of the gate is
# proving the binding works on the musl libc family it was cross-compiled for.
set -euo pipefail

log() { echo "smoke-test-musl-node-binding: $*" >&2; }
die() {
  log "$*"
  exit 1
}

STAGED_DIR="${1:?usage: $0 <staged-platform-dir-containing-.node-and-.so-files>}"
[ -d "$STAGED_DIR" ] || die "staged platform directory not found: $STAGED_DIR"
# `docker -v` rejects a relative source as a volume NAME ("includes invalid
# characters for a local volume name"), so this must be absolute before the
# mount below. Matches smoke-test-musl-python-wheel.sh, which already does it. ~keep
STAGED_DIR="$(cd "$STAGED_DIR" && pwd)"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$REPO_ROOT/scripts/ci/fixtures/musl-python-smoke.pdf"
INDEX_JS="$REPO_ROOT/crates/xberg-node/index.js"
SMOKE_SCRIPT="$REPO_ROOT/scripts/ci/smoke_test_node_binding.js"
EXPECTED_TEXT="XBERG MUSL SMOKE 490"

[ -f "$FIXTURE" ] || die "smoke-test fixture missing: $FIXTURE"
[ -f "$INDEX_JS" ] || die "napi-rs index.js missing: $INDEX_JS"
[ -f "$SMOKE_SCRIPT" ] || die "smoke-test script missing: $SMOKE_SCRIPT"

NODE_FILE="$(find "$STAGED_DIR" -maxdepth 1 -name '*.node' | head -n1)"
[ -n "$NODE_FILE" ] || die "no .node file found under $STAGED_DIR"
NODE_FILE_NAME="$(basename "$NODE_FILE")"

# alpine:3.24.3 matches docker/Dockerfile.musl-node's build image, so this is
# the same musl libc family the binding was cross-compiled for.
docker run --rm \
  -v "$STAGED_DIR:/native:ro" \
  -v "$FIXTURE:/fixture.pdf:ro" \
  -v "$INDEX_JS:/smoke/index.js:ro" \
  -v "$SMOKE_SCRIPT:/smoke/smoke_test_node_binding.js:ro" \
  -e NAPI_RS_NATIVE_LIBRARY_PATH="/native/${NODE_FILE_NAME}" \
  -e EXPECTED_TEXT="$EXPECTED_TEXT" \
  alpine:3.24.3 sh -euc '
    apk add --no-cache nodejs >/dev/null
    # Same libstdc++ skew the Elixir gate hits: the vendored libonnxruntime.so.1 comes
    # from Alpine edge (docker/Dockerfile.musl-node upgrades libstdc++ before compiling)
    # and relocates against symbols GCC 15 added. Node links libstdc++ itself, so the
    # process already has 3.22 14.2.0 mapped under that soname before the addon is
    # dlopened and the $ORIGIN copy beside the .node is never consulted. ~keep
    apk add --no-cache --upgrade libstdc++ \
      --repository=https://dl-cdn.alpinelinux.org/alpine/edge/main >/dev/null
    node /smoke/smoke_test_node_binding.js /smoke/index.js /fixture.pdf "${EXPECTED_TEXT}"
  '
