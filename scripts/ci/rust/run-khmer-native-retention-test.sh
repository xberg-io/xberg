#!/usr/bin/env bash
set -euo pipefail

features="pdf,pdf-native,ocr,ocr-pipeline,tokio-runtime,bundle-tessdata-eng,language-detection,html,image-encode"
test_case="automatic_english_ocr_preserves_known_native_khmer_even_when_mapping_is_partial"
cargo_args=(test --locked -p xberg --no-default-features --features "$features"
  --test pdf_khmer_native_retention_2536)
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
export HF_HUB_OFFLINE=1
export HUGGINGFACE_HUB_OFFLINE=1

listed="$(cargo "${cargo_args[@]}" -- --exact "$test_case" --list --color never)"
printf '%s\n' "$listed"
grep -qxF "$test_case: test" <<<"$listed"
grep -qxF "1 test, 0 benchmarks" <<<"$listed"
cargo "${cargo_args[@]}" -- --exact "$test_case" --color never
