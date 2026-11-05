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
if ! grep -qF 'test result: ok. 4 passed; 0 failed; 0 ignored;' "$log"; then
  echo "Expected four executed PDF orientation tests" >&2
  exit 1
fi
