#!/usr/bin/env bash
#MISE description="Run the V3 cross-runtime tests and fail when the number of passed tests is not the known number"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
expected_tests=57

set +e
tests=scripts/ci/test_v3_cross_runtime.py
output="$(cd "$root" && uv run --frozen --only-dev --with numpy==2.4.6 python -m pytest -q "$tests" 2>&1)"
status=$?
set -e
printf '%s\n' "$output"
if [ "$status" -ne 0 ]; then
  exit "$status"
fi
passed="$(printf '%s\n' "$output" | grep -Eo '[0-9]+ passed' | tail -n 1 | grep -Eo '[0-9]+' || true)"
if [ "${passed:-0}" -ne "$expected_tests" ]; then
  printf 'Expected %s passed V3 cross-runtime tests, observed %s; update expected_tests when adding tests\n' \
    "$expected_tests" "${passed:-0}" >&2
  exit 1
fi
