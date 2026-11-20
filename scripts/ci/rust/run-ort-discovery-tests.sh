#!/usr/bin/env bash
set -euo pipefail

runtime="${ORT_DYLIB_PATH:?set by the ONNX Runtime setup action}"
candidate=/usr/local/lib/libonnxruntime.so
test -f "$runtime"
[[ "$runtime" == /* && ! -e "$candidate" && ! -L "$candidate" ]]
test ! -e /usr/lib/libonnxruntime.so
sudo ln -s -- "$runtime" "$candidate"
log=$(mktemp)
trap 'rm -f -- "$log"; if [[ "$(readlink "$candidate")" == "$runtime" ]]; then sudo rm -- "$candidate"; fi' EXIT

run_exact() {
  local features="$1" test="$2"
  cargo test --locked -p xberg --no-default-features --features "$features" --lib "$test" -- \
    --exact --include-ignored --color never | tee "$log"
  grep -Fxq "test $test ... ok" "$log"
  grep -Fq 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log"
}

run_exact sceptre-ocr-ort,ort-dynamic \
  ort_discovery::runtime_route_tests::standalone_sceptre_preloads_before_reading_the_image
run_exact auto-rotate,ort-dynamic \
  ort_discovery::runtime_route_tests::environment_preloads_for_consumers_without_sceptre
for test in valid_explicit_path_takes_precedence_over_an_invalid_candidate \
  missing_candidate_leaves_bare_name_to_the_loader discovered_runtime_loads_without_overwriting_the_environment \
  broken_candidate_reports_a_typed_error_before_ort_initialization; do
  run_exact sceptre-ocr-ort,ort-dynamic "ort_discovery::dynamic_sceptre_tests::$test"
done
