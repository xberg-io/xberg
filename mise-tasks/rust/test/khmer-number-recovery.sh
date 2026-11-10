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
  provisioned_recognition_refuses_painted_latin_lookalikes
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
  should_require_proven_page_context_for_numeric_recovery
  should_keep_recovered_page_text_when_final_rendering_has_no_page
  should_preserve_multiline_authoritative_ocr_text_when_inserting_a_number
  should_decline_unqualified_primary_languages_before_numeric_recovery
  should_refuse_when_the_provisioned_crop_model_disappears
  should_refuse_painted_latin_lookalikes_in_a_claimed_khmer_crop
  should_defer_missing_model_warnings_until_the_primary_page_is_accepted
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

shape_args=(test --locked -p xberg --no-default-features --features "$features" --lib number_recovery::shape_proof::)
shape_names=(
  tests::should_corroborate_genuine_khmer_zero_from_actual_font_program
  tests::should_corroborate_genuine_shape_with_native_canvas_rounding
  tests::should_refuse_painted_latin_o_under_hidden_khmer_zero
  tests::should_refuse_painted_ascii_zero_under_hidden_khmer_zero
  tests::should_corroborate_unchanged_outline_despite_wrong_to_unicode
  tests::should_refuse_blank_source_and_zero_render_budget
  tests::should_decline_preparation_without_byte_budget
  tests::should_decline_unsupported_font_before_native_character_decode
  tests::should_decline_unbounded_unicode_ranges_before_native_character_decode
  tests::should_decline_ambiguous_physical_code_even_when_claim_is_unchanged
  tests::should_decline_cyclic_or_excessive_font_object_graphs
  tests::should_decline_nonbyte_cmap_formats_before_native_font_loading
  tests::should_decline_oversized_raw_and_compressed_font_streams
  tests::should_decline_ambiguous_fonts_and_transformed_glyphs
  tests::should_refuse_rebound_candidates_anisotropic_rasters_and_extra_source_ink
  tests::should_decline_long_claim_with_unused_alternate_byte_before_native
  tests::should_decline_multiscalar_and_surrogate_claims_before_native
  tests::should_decline_real_font_cmap_ranges_and_truncation_before_native
  tests::should_decline_real_font_allocator_maxima_and_composites_before_native
  tests::should_decline_real_font_actual_outline_overflow_despite_small_maxp
  tests::should_reserve_font_working_bytes_for_nonzero_render_budget
  tests::should_decline_broken_loca_and_actual_instruction_lengths_before_native
  tests::should_decline_unaudited_form_font_resources_before_native
  tests::should_decline_nonzero_preflight_budget_below_native_font_cache_reservation
  tests::should_decline_complete_format12_billion_code_group_before_native
  source_tests::should_decline_inline_actual_text_before_native_byte_reconstruction
  source_tests::should_decline_named_escaped_actual_text_properties_before_native
  source_tests::should_decline_all_marked_content_and_unknown_text_producers
  source_tests::should_decline_oversized_compressed_contents_before_native
  source_tests::should_decline_nested_content_arrays_before_recursive_parser
  source_tests::should_count_repeated_shows_before_native_overlap_deduplication
  source_tests::should_decline_huge_font_names_before_character_string_allocation
  source_tests::should_decline_real_unused_latin_o_byte_selected_by_actual_text_utf8
  source_tests::should_reserve_source_peak_before_nonzero_small_budget_extraction
  source_tests::should_decline_unsupported_contents_filters_and_parameters
  source_tests::should_decline_cyclic_and_excessive_indirect_contents_arrays
  source_tests::should_decline_tagged_pages_and_external_graphics_state_resources
  source_tests::should_decline_excessive_source_operations_and_literal_nesting
  source_tests::should_prepare_genuine_source_through_the_single_bounded_entry
  source_tests::should_accept_bounded_indirect_contents_arrays_and_count_tj_spaces
  source_tests::should_decline_actual_text_in_both_source_and_shape_entry_points
  source_tests::should_bound_font_aliases_and_descriptor_names_before_loading
  source_tests::should_bound_total_decoded_contents_across_indirect_streams
  source_tests::should_decline_array_comments_that_conceal_actual_text_from_admission
  source_tests::should_decline_array_comments_that_hide_character_allocation
  source_tests::should_decline_terminal_comments_that_replay_actual_text_at_eof
  source_tests::should_decline_terminal_comments_that_hide_character_allocation_at_eof
)
expected_shapes="${#shape_names[@]}"
shape_listed="$(cargo "${shape_args[@]}" -- --include-ignored --list --color never)"
printf '%s\n' "$shape_listed"
observed_shapes="$(awk '/: test$/ { count++ } END { print count + 0 }' <<<"$shape_listed")"
[ "$observed_shapes" -eq "$expected_shapes" ] || {
  echo "Expected $expected_shapes collected glyph-shape controls; observed $observed_shapes" >&2
  exit 1
}
for name in "${shape_names[@]}"; do
  grep -qxF "extractors::pdf::ocr::number_recovery::shape_proof::$name: test" <<<"$shape_listed" || {
    echo "Required glyph-shape control was not collected: $name" >&2
    exit 1
  }
done
shape_log="$(mktemp "${TMPDIR:-/tmp}/xberg-2408-shapes.XXXXXX")"
cargo "${shape_args[@]}" -- --include-ignored --test-threads=1 --color never 2>&1 | tee "$shape_log"
grep -qF "test result: ok. $expected_shapes passed; 0 failed; 0 ignored;" "$shape_log" || {
  echo "Expected $expected_shapes executed glyph-shape controls with no ignored tests; inspect $shape_log" >&2
  exit 1
}
echo "Native glyph-shape controls: expected $expected_shapes, passed $expected_shapes; log $shape_log"
