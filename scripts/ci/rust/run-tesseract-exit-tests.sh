#!/usr/bin/env bash
set -euo pipefail

# The exit cleanup tests build only with `bundle-tessdata-eng` (`required-features` in
# crates/xberg-tesseract/Cargo.toml). A test leg without that feature skips both files and still
# passes, so this check names the features itself and requires all three tests to run and pass. ~keep
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"
[ -f "$REPO_ROOT/Cargo.toml" ] || {
  echo "Missing repository Cargo.toml" >&2
  exit 1
}
cd "$REPO_ROOT"

expected=3
cargo_args=(test --locked -p xberg-tesseract --features build-tesseract,bundle-tessdata-eng
  --test exit_cleanup_leak --test exit_cleanup_race)

listed="$(cargo "${cargo_args[@]}" -- --list --color never)"
printf '%s\n' "$listed"
count="$(awk '/: test$/ { count++ } END { print count + 0 }' <<<"$listed")"
if [ "$count" -ne "$expected" ]; then
  echo "Tesseract exit cleanup tests: expected $expected collected tests, observed $count" >&2
  exit 1
fi

ran="$(cargo "${cargo_args[@]}" --no-fail-fast -- --color never 2>&1 | tee /dev/stderr)"
passed="$(awk '/^test result: ok\./ { sum += $4 } END { print sum + 0 }' <<<"$ran")"
if [ "$passed" -ne "$expected" ]; then
  echo "Tesseract exit cleanup tests: expected $expected passed tests, observed $passed" >&2
  exit 1
fi
echo "Tesseract exit cleanup tests expected $expected, passed $passed"
