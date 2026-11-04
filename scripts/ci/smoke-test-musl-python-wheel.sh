#!/usr/bin/env bash
# Installs a built musl Python wheel into a genuine Alpine (musl libc) container
# and runs a real extraction against a known fixture, asserting the exact
# expected text comes back. See scripts/ci/smoke_test_python_wheel.py for why an
# import-only check is not sufficient here (xberg-io/xberg#490).
#
# Deliberately does NOT run on the GitHub Actions host directly: the host is
# glibc (ubuntu-latest / ubuntu-24.04-arm), and the entire point of the gate is
# proving the wheel works on the musl libc family it was cross-compiled for --
# a glibc host resolving the wheel's dynamic deps would prove nothing about
# musl compatibility.
set -euo pipefail

log() { echo "smoke-test-musl-python-wheel: $*" >&2; }
die() {
  log "$*"
  exit 1
}

WHEEL="${1:?usage: $0 <path-to-wheel>}"
[ -f "$WHEEL" ] || die "wheel not found: $WHEEL"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$REPO_ROOT/scripts/ci/fixtures/musl-python-smoke.pdf"
SMOKE_SCRIPT="$REPO_ROOT/scripts/ci/smoke_test_python_wheel.py"
EXPECTED_TEXT="XBERG MUSL SMOKE 490"

[ -f "$FIXTURE" ] || die "smoke-test fixture missing: $FIXTURE"
[ -f "$SMOKE_SCRIPT" ] || die "smoke-test script missing: $SMOKE_SCRIPT"

WHEEL_DIR="$(cd "$(dirname "$WHEEL")" && pwd)"
WHEEL_NAME="$(basename "$WHEEL")"

# alpine:3.24.3 matches docker/Dockerfile.musl-python's build image, so this is
# the same musl libc family (and the same Alpine python3) the wheel was
# cross-compiled for.
docker run --rm \
  -v "$WHEEL_DIR/$WHEEL_NAME:/wheel/$WHEEL_NAME:ro" \
  -v "$FIXTURE:/fixture.pdf:ro" \
  -v "$SMOKE_SCRIPT:/smoke_test_python_wheel.py:ro" \
  -e WHEEL_NAME="$WHEEL_NAME" \
  -e EXPECTED_TEXT="$EXPECTED_TEXT" \
  alpine:3.24.3 sh -euc '
    apk add --no-cache python3 py3-pip >/dev/null
    python3 -m venv /venv
    /venv/bin/pip install --no-cache-dir --quiet "/wheel/${WHEEL_NAME}"
    /venv/bin/python3 /smoke_test_python_wheel.py /fixture.pdf "${EXPECTED_TEXT}"
  '
