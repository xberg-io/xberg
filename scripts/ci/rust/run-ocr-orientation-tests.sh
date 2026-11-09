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

# The strict two-table and heading-plus-table certificates are exercised on the direct Tesseract route.
direct_features=pdf,ocr,auto-rotate-tract,bundle-tessdata-eng,tokio-runtime
executed=0
for target_case in \
  'ocr_multitable_orientation_2497 implicit_rotation_recovers_two_tables_when_primary_iterator_words_cover_its_output' \
  'ocr_multitable_orientation_2497 implicit_sideways_heading_recovers_exact_source_and_table_bounds' \
  'ocr_mixed_tables_2497 automatic_two_tables_and_heading_preserve_every_source_word' \
  'ocr_mixed_tables_2497 automatic_upright_tables_stay_in_source_drawing_regions'; do
  read -r test_target case_name <<<"$target_case"
  cargo test --locked -p xberg --no-default-features --features "$direct_features" \
    --test "$test_target" -- --exact "$case_name" --color never 2>&1 | tee "$log"
  if ! grep -qF "test $case_name ... ok" "$log" ||
    ! grep -qF 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log"; then
    echo "Expected the pinned orientation test $case_name to execute" >&2
    exit 1
  fi
  executed=$((executed + 1))
done
test "$executed" -eq 4
