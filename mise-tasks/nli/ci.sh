#!/usr/bin/env bash
#MISE description="Verify NLI contracts and compile/lint the optional runtime"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/nli_qualification/Cargo.toml"
expected_tests=22
export ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
cargo fetch --locked --manifest-path "$manifest"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
mise run nli:test 2>&1 | tee "$log"
passed="$(awk '/test result: ok\./ {
  for (i=1; i<=NF; i++) if ($i == "passed;") sum += $(i-1)
} END { print sum + 0 }' "$log")"
if [ "$passed" -ne "$expected_tests" ]; then
  printf 'Expected %s executed NLI tests, observed %s; update expected_tests when adding tests\n' \
    "$expected_tests" "$passed" >&2
  exit 1
fi
cargo check --offline --locked --manifest-path "$manifest" --features runtime
cargo clippy --offline --locked --manifest-path "$manifest" --all-targets --features runtime -- -D warnings
