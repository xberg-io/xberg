#!/usr/bin/env bash
#MISE description="Audit and verify NLI contracts and the optional runtime"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/nli_qualification/Cargo.toml"
expected_tests=27
export ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
cargo fetch --locked --manifest-path "$manifest"
if [ "${CARGO_NET_OFFLINE:-false}" = true ]; then
  mise run nli:audit --offline
else
  mise run nli:audit
fi
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

run_tests() {
  local label="$1"
  shift
  mise run nli:test "$@" 2>&1 | tee "$log"
  local passed
  passed="$(awk '/test result: ok\./ {
    for (i=1; i<=NF; i++) if ($i == "passed;") sum += $(i-1)
  } END { print sum + 0 }' "$log")"
  if [ "$passed" -ne "$expected_tests" ]; then
    printf 'Expected %s executed NLI tests, observed %s; update expected_tests when adding tests\n' \
      "$expected_tests" "$passed" >&2
    exit 1
  fi
  printf 'NLI %s: %s tests passed (%s expected)\n' "$label" "$passed" "$expected_tests"
}

run_tests portable
run_tests runtime --features runtime
cargo check --offline --locked --manifest-path "$manifest" --features runtime
cargo clippy --offline --locked --manifest-path "$manifest" --all-targets --features runtime -- -D warnings
