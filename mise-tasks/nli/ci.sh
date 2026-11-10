#!/usr/bin/env bash
#MISE description="Audit and verify NLI contracts and the optional runtime"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/nli_qualification/Cargo.toml"
expected_portable=28
expected_runtime=28
export ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
cargo fetch --locked --manifest-path "$manifest"
mise run nli:audit -- --offline
mise run nli:audit:control
mise run nli:setup:check
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

run_tests() {
  local label="$1" expected="$2"
  shift 2
  mise run nli:test -- "$@" 2>&1 | tee "$log"
  local passed
  passed="$(awk '/test result: ok\./ {
    for (i=1; i<=NF; i++) if ($i == "passed;") sum += $(i-1)
  } END { print sum + 0 }' "$log")"
  if [ "$passed" -ne "$expected" ]; then
    printf 'Expected %s executed NLI tests, observed %s; update the mode count when adding tests\n' \
      "$expected" "$passed" >&2
    exit 1
  fi
  local mode_test
  if [ "$label" = portable ]; then
    mode_test=run_without_runtime_feature_is_an_explicit_failure_not_skipped_success
  else
    mode_test=portable_admission_agrees_with_sdk_constructor_on_edge_configurations
  fi
  if ! grep -Fq "test $mode_test ... ok" "$log"; then
    printf 'Expected the %s feature-mode sentinel test to execute\n' "$label" >&2
    exit 1
  fi
  printf 'NLI %s: %s tests passed (%s expected)\n' "$label" "$passed" "$expected"
}

run_tests portable "$expected_portable"
run_tests runtime "$expected_runtime" --features runtime
cargo check --offline --locked --manifest-path "$manifest" --features runtime
cargo clippy --offline --locked --manifest-path "$manifest" --all-targets --features runtime -- -D warnings
