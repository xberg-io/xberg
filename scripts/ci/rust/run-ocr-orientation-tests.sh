#!/usr/bin/env bash
set -euo pipefail

repo=models--xberg-io--paddleocr-onnx-models
revision=bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8
file=v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx
export XBERG_REQUIRE_MODELS=1
export XBERG_ORIENTATION_TEST_MODEL="${HF_HUB_CACHE:?Pinned model cache required}/${repo}/snapshots/${revision}/${file}"
test -s "${XBERG_ORIENTATION_TEST_MODEL}"

log=$(mktemp)
trap 'rm -f "$log"' EXIT
cargo test --locked -p xberg --no-default-features --features "${TRACT_FEATURES:?Tract-only features required}" \
  --test ocr_auto_rotate -- implicit_ --color never 2>&1 | tee "$log"
if ! grep -qF 'test result: ok. 7 passed; 0 failed; 0 ignored;' "$log"; then
  echo "Expected seven executed PDF orientation tests" >&2
  exit 1
fi

# Keep the Tract certificate and the Paddle-enabled detached pipeline in separate feature closures.
direct_features=pdf,ocr,auto-rotate-tract,bundle-tessdata-eng,tokio-runtime
paddle_features=pdf,ocr,ocr-pipeline,paddle-ocr-ort,bundle-tessdata-eng,tokio-runtime
executed=0
run_exact() {
  local features=$1 test_target=$2 case_name=$3
  cargo test --locked -p xberg --no-default-features --features "$features" \
    --test "$test_target" -- --exact "$case_name" --color never 2>&1 | tee "$log"
  if ! grep -qF "test $case_name ... ok" "$log" ||
    ! grep -qF 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log"; then
    echo "Expected the pinned orientation test $case_name to execute" >&2
    exit 1
  fi
  executed=$((executed + 1))
}
for target_case in \
  'ocr_multitable_orientation_2497 implicit_rotation_recovers_two_tables_when_primary_iterator_words_cover_its_output' \
  'ocr_multitable_orientation_2497 implicit_sideways_heading_recovers_exact_source_and_table_bounds' \
  'ocr_multitable_orientation_2497 implicit_rotation_keeps_a_noise_read_when_no_tables_are_recovered' \
  'ocr_mixed_tables_2497 automatic_two_tables_and_heading_preserve_every_source_word' \
  'ocr_mixed_tables_2497 automatic_upright_tables_stay_in_source_drawing_regions' \
  'ocr_mixed_tables_2497 independent_source_bounds_refuse_degenerate_subboxes' \
  'ocr_mixed_tables_2497 single_table_heading_recovers_both_sideways_source_poses' \
  'ocr_mixed_tables_2497 mixed_tables_keep_primary_words_outside_the_corrected_proof'; do
  read -r test_target case_name <<<"$target_case"
  run_exact "$direct_features" "$test_target" "$case_name"
done
run_exact "$paddle_features" ocr_mixed_tables_2497 detached_mixed_tables_keep_source_words_structure_and_frames
test "$executed" -eq 9
