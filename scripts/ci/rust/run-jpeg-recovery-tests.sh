#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"
[ -f "$REPO_ROOT/Cargo.toml" ] || {
  echo "Missing repository Cargo.toml" >&2
  exit 1
}
cd "$REPO_ROOT"

features="${1:?Rust feature list required}"
no_defaults="${2:-false}"
cargo_args=(test --locked -p xberg --features "$features" --lib)
case "$no_defaults" in
true) cargo_args+=(--no-default-features) ;;
false) ;;
*)
  echo "Expected no-default-features true or false" >&2
  exit 1
  ;;
esac
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
export HF_HUB_OFFLINE=1
log_dir="$(mktemp -d)"
trap 'rm -rf "$log_dir"' EXIT

passed=0
run_group() {
  local filter="$1" expected="$2" observed
  cargo "${cargo_args[@]}" "$filter" -- --ignored --list --color never 2>&1 | tee "$log_dir/list.log"
  observed="$(awk '/: test$/ { count++ } END { print count + 0 }' "$log_dir/list.log")"
  if [ "$observed" -ne "$expected" ]; then
    echo "JPEG recovery '$filter': expected $expected collected tests, observed $observed" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" "$filter" -- --ignored --test-threads=1 --color never 2>&1 | tee "$log_dir/run.log"
  if ! grep -qF "test result: ok. $expected passed; 0 failed; 0 ignored;" "$log_dir/run.log"; then
    echo "JPEG recovery '$filter': expected $expected executed tests with none ignored" >&2
    exit 1
  fi
  passed=$((passed + expected))
}

run_group 'english_recovery_tests::small_english_' 2
run_group 'encoded_pipeline::builtin_pipeline_png_' 3
run_group 'english_recovery_tests::large::large_english_' 2
[ "$passed" -eq 7 ] || exit 1
echo "JPEG recovery model tests expected 7, passed $passed"
