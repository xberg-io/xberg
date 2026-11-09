#!/usr/bin/env bash
#MISE description="Qualify native-number recovery with a pinned, already provisioned Khmer model"
set -euo pipefail

model="${XBERG_2408_KHM_MODEL:?Provide the pinned Khmer traineddata; this task never downloads models}"
python3 - "$model" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

model = Path(sys.argv[1]).read_bytes()
expected = "47f110575341b322052f3becbefee61a3ecf1ef549352b5d4d33d28afe30d099"
if len(model) != 1446926 or sha256(model).hexdigest() != expected:
    raise SystemExit("Khmer qualification model size/SHA256 mismatch")
print("Verified pinned Khmer qualification model")
PY

export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
features="pdf,ocr,tokio-runtime,bundle-tessdata-eng"
args=(test --locked -p xberg --no-default-features --features "$features" --test pdf_khmer_number_2408)
expected_names=(
  a_number_in_a_blank_source_crop_is_not_recovered
  deskewed_primary_does_not_promote_a_number_in_the_original_source_frame
  english_only_keeps_the_unverified_ocr_read
  forced_english_ocr_does_not_promote_native_numbers
  installing_a_model_does_not_reuse_an_english_only_cached_result
  overlapping_native_claims_cannot_multiply_one_painted_number
  provisioned_recognition_still_refuses_false_native_digits
  recovers_the_painted_zero_once_at_its_source_position
  recovery_keeps_prose_order_without_guessing_inline_word_boundaries
  refuses_a_hidden_digit_not_painted_by_the_scan
  refuses_wrong_unicode_even_when_font_glyph_and_scan_are_unchanged
  same_scan_has_different_hidden_digits
)
expected="${#expected_names[@]}"
listed="$(cargo "${args[@]}" -- --skip helpers:: --include-ignored --list --color never)"
printf '%s\n' "$listed"
observed="$(awk '/: test$/ { count++ } END { print count + 0 }' <<<"$listed")"
[ "$observed" -eq "$expected" ] || {
  echo "Expected $expected collected numeric-recovery tests; observed $observed" >&2
  exit 1
}
for name in "${expected_names[@]}"; do
  grep -qxF "$name: test" <<<"$listed" || {
    echo "Required numeric-recovery test was not collected: $name" >&2
    exit 1
  }
done
log="$(mktemp "${TMPDIR:-/tmp}/xberg-2408.XXXXXX")"
cargo "${args[@]}" -- --skip helpers:: --include-ignored --test-threads=1 --color never 2>&1 | tee "$log"
grep -qF "test result: ok. $expected passed; 0 failed; 0 ignored;" "$log" || {
  echo "Expected $expected executed tests with no ignored tests; inspect $log" >&2
  exit 1
}
echo "Native-number recovery: expected $expected, passed $expected; log $log"

unit_args=(test --locked -p xberg --no-default-features --features "$features" --lib number_recovery::tests::)
unit_names=(
  should_keep_authoritative_text_whose_located_segments_are_incomplete
  should_not_invent_an_owner_when_two_ocr_lines_overlap
  should_not_duplicate_a_number_ocr_already_read_on_the_line
  should_keep_two_independently_verified_occurrences_on_one_line
  should_refuse_overlapping_claims_for_one_painted_occurrence
  should_keep_original_prose_when_numeric_order_is_not_proved
  should_keep_ocr_junk_but_add_an_unstyled_verified_number
  should_keep_adjacent_native_digits_in_one_numeric_candidate
  should_honor_the_actual_forced_page_number
  should_refuse_crops_outside_the_page_or_live_memory_budget
  should_bind_positive_scale_only_evidence_to_the_selected_geometry
  should_refuse_an_oversized_selected_read_proof
  should_normalize_only_spacing_in_the_recognized_numeric_run
  should_reuse_one_lazy_processor_for_independent_page_verifications
)
expected_units="${#unit_names[@]}"
unit_listed="$(cargo "${unit_args[@]}" -- --include-ignored --list --color never)"
printf '%s\n' "$unit_listed"
observed_units="$(awk '/: test$/ { count++ } END { print count + 0 }' <<<"$unit_listed")"
[ "$observed_units" -eq "$expected_units" ] || {
  echo "Expected $expected_units collected numeric-recovery unit controls; observed $observed_units" >&2
  exit 1
}
for name in "${unit_names[@]}"; do
  grep -qxF "extractors::pdf::ocr::number_recovery::tests::$name: test" <<<"$unit_listed" || {
    echo "Required numeric-recovery unit control was not collected: $name" >&2
    exit 1
  }
done
unit_log="$(mktemp "${TMPDIR:-/tmp}/xberg-2408-units.XXXXXX")"
cargo "${unit_args[@]}" -- --include-ignored --test-threads=1 --color never 2>&1 | tee "$unit_log"
grep -qF "test result: ok. $expected_units passed; 0 failed; 0 ignored;" "$unit_log" || {
  echo "Expected $expected_units executed unit controls with no ignored tests; inspect $unit_log" >&2
  exit 1
}
echo "Native-number unit controls: expected $expected_units, passed $expected_units; log $unit_log"
