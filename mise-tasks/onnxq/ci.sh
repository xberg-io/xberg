#!/usr/bin/env bash
#MISE description="Test the held-out scoring contracts, run the negative controls and lint the leaf"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/onnx_qualification/Cargo.toml"
export ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
cargo fetch --locked --manifest-path "$manifest"

log="$(mktemp)"
controls="$(mktemp -d)/controls.json"
trap 'rm -f "$log"; rm -rf "$(dirname "$controls")"' EXIT

mise run onnxq:test 2>&1 | tee "$log"
passed="$(awk '/test result: ok\./ {
  for (i=1; i<=NF; i++) if ($i == "passed;") sum += $(i-1)
} END { print sum + 0 }' "$log")"
expected=26
if [ "$passed" -ne "$expected" ]; then
  printf 'Expected %s executed onnx-qualification tests, observed %s; update the count when adding tests\n' \
    "$expected" "$passed" >&2
  exit 1
fi
if ! grep -Fq "test refuses_empty_corpus_and_wrong_counts_for_the_intended_reason ... ok" "$log"; then
  printf 'Expected the portable contract sentinel test to execute\n' >&2
  exit 1
fi

cargo run --offline --locked --quiet --manifest-path "$manifest" -- controls "$controls"
python3 - "$controls" <<'PY'
import json, sys

report = json.load(open(sys.argv[1]))
expected = 14
passed = sum(1 for control in report["controls"] if control["control_passed"])
if len(report["controls"]) != expected or passed != expected or report["failed"] != 0:
    raise SystemExit(
        f"negative controls: {passed}/{expected} fired, report failed={report['failed']}"
    )
print(f"negative controls: {passed}/{expected} mutations rejected for the intended reason")
PY

cargo check --offline --locked --manifest-path "$manifest"
cargo clippy --offline --locked --manifest-path "$manifest" --all-targets -- -D warnings
