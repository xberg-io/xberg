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
for case_name in \
  implicit_rotation_retains_primary_when_two_tables_lack_complete_primary_evidence \
  implicit_sideways_heading_recovers_exact_source_and_table_bounds; do
  cargo test --locked -p xberg --no-default-features --features "$direct_features" \
    --test ocr_multitable_orientation_2497 -- --exact "$case_name" --color never 2>&1 | tee "$log"
  if ! grep -qF "test $case_name ... ok" "$log" ||
    ! grep -qF 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log"; then
    echo "Expected the pinned orientation test $case_name to execute" >&2
    exit 1
  fi
done
