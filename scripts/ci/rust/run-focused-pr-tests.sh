#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
suite="${1:?expected pdf-ocr, pdf-sparse-paddle, office-pages, api-jobs, pdf-table-paint or pdf-regression-pages}"
case "$suite" in
pdf-ocr) features="pdf,ocr,tokio-runtime,bundle-tessdata-eng,layout-types" ;;
pdf-sparse-paddle) features="pdf,ocr,tokio-runtime,bundle-tessdata-eng,layout-types,paddle-ocr-tract" ;;
office-pages) features="office,excel,tokio-runtime" ;;
api-jobs) features="api,tokio-runtime" ;;
pdf-table-paint) features="pdf,layout-detection" ;;
pdf-regression-pages) features="pdf" ;;
*)
  printf 'Unknown focused test suite: %s\n' "$suite" >&2
  exit 2
  ;;
esac

log_dir="$(mktemp -d)"
trap 'rm -rf "$log_dir"' EXIT
export XBERG_CACHE_DIR="$log_dir/xberg-cache"
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
export HF_HUB_OFFLINE=1
passed=0

run_exact() {
  local test_name="$1"
  shift
  local ignored=""
  if [[ "${1:-}" == --ignored ]]; then
    ignored="$1"
    shift
  fi
  local -a cargo_args
  if [[ "${1:-}" == --native-pdf ]]; then
    shift
    cargo_args=(test --locked -p xberg-native-pdf "$@")
  elif [[ "${1:-}" == --cli ]]; then
    shift
    cargo_args=(test --locked -p xberg-cli --no-default-features --features pdf-ocr --bin xberg "$@")
  else
    cargo_args=(test --locked -p xberg --no-default-features --features "$features" "$@")
  fi
  cargo "${cargo_args[@]}" -- --exact "$test_name" ${ignored:+"$ignored"} --list --color never 2>&1 | tee "$log_dir/list.log"
  if ! grep -Fxq "$test_name: test" "$log_dir/list.log" ||
    ! grep -Fxq '1 test, 0 benchmarks' "$log_dir/list.log"; then
    printf 'Expected exactly one collected test: %s\n' "$test_name" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" -- --exact "$test_name" ${ignored:+"$ignored"} --color never 2>&1 | tee "$log_dir/run.log"
  if ! grep -Fxq "test $test_name ... ok" "$log_dir/run.log" ||
    ! grep -Fq 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log_dir/run.log"; then
    printf 'Expected exactly one executed test: %s\n' "$test_name" >&2
    exit 1
  fi
  passed=$((passed + 1))
}

check_mixed_decision_layer_handoffs() {
  # Recovery can fall back to decoded PDF words, and it requires Tokio. Check both callers here. ~keep
  python3 - <<'PYTHON'
from pathlib import Path
import re

source = Path("crates/xberg/src/extractors/pdf/ocr/pipeline.rs").read_text()
lines = source.splitlines()


def method_body(header: str) -> str:
    start = next(index for index, line in enumerate(lines) if line.startswith(header))
    end = next(index for index in range(start + 1, len(lines)) if lines[index] == "    }")
    return "\n".join(lines[start : end + 1])


def free_body(header: str) -> str:
    start = next(index for index, line in enumerate(lines) if line.startswith(header))
    end = next(index for index in range(start + 1, len(lines)) if lines[index] == "}")
    return "\n".join(lines[start : end + 1])


entry = free_body("async fn extract_mixed_ocr_native_with_layout_inputs(")
assert re.search(r"run_page_batches\(\)\.await\?;\s*\n\s*context\.finish\(\)", entry), (
    "the dispatcher must run the page batches before finishing"
)

hints_builder = method_body("    fn hints_for_pipeline_page(")
tokio = method_body("    async fn run_pipeline_pages_async(")
collector = method_body("    async fn collect_pipeline_reads<F>(")
sequential = method_body("    async fn run_pipeline_pages_sequential(")
sequential_backend = method_body("    async fn run_single_backend_pages_sequential(")
finish_backend = method_body("    async fn finish_single_backend_read(")

assert re.search(r"decision_layers:\s*self\.page_layers\.clone\(\),", hints_builder), (
    "the shared page hints must supply the decision layers"
)
assert re.search(r"self\.hints_for_pipeline_page\(idx, None\)", tokio), (
    "the sparse trial must use the same page hints as regular detached OCR"
)
assert re.search(r"let hints =\s*self\.hints_for_pipeline_page\(\s*page_idx,", tokio), (
    "the Tokio route's regular page hints must come from the shared builder"
)
assert re.search(r"let hints =\s*self\.hints_for_pipeline_page\(\s*retry_idx,", collector), (
    "the Tokio retry-render hints must come from the shared builder"
)
assert re.search(r"\blet mut page_ocr_hints = hints;", tokio), (
    "the Tokio route must build its page hints from the shared builder"
)
assert re.search(r"spawn_page\(&mut join_set, page_idx, image_arc, hints, page_detection\);", tokio), (
    "the Tokio route must pass the shared builder's hints to the page task"
)
assert re.search(r"spawn_page\(join_set, retry_idx, image_arc, hints, page_detection\);", collector), (
    "the Tokio retry requeue must pass the shared builder's hints to the page task"
)
assert re.search(r"Some\(page_ocr_hints\),\s*idx,\s*Some\(source\),", tokio), (
    "the Tokio mixed OCR route must pass the hints and the page index to detached OCR"
)

assert re.search(r"decision_layers:\s*self\.page_layers\.clone\(\),", sequential), (
    "the sequential mixed OCR route must supply the decision layers"
)
assert re.search(r"Some\(page_ocr_hints\),\s*\*page_idx,\s*Some\(source\),", sequential), (
    "the sequential mixed OCR route must pass the hints and the page index to detached OCR"
)

assert re.search(r"record_single_backend_read\(", finish_backend) and re.search(
    r"GeometryEvidence::concurrent\(\)", finish_backend
), "the concurrent backend route must run the geometry and language passes"
assert re.search(r"record_single_backend_read\(", sequential_backend) and re.search(
    r"GeometryEvidence::sequential\(\)", sequential_backend
), "the sequential backend route must skip the geometry and language passes"
assert "GeometryEvidence::sequential()" not in finish_backend, (
    "the concurrent route must not use the sequential evidence policy"
)
assert "GeometryEvidence::concurrent()" not in sequential_backend, (
    "the sequential route must not use the concurrent evidence policy"
)
print("Tokio mixed OCR decision-layer handoff verified")
print("sequential mixed OCR decision-layer handoff verified")
print("direct-backend geometry evidence routing verified")
PYTHON
}

run_pdf_cases() {
  run_exact scanned_prose_rejoins_witnessed_line_end_hyphens --test pdf_scanned_hyphenation_3170
  run_exact scanned_lexical_wrap_keeps_hyphen_despite_closed_spelling --test pdf_scanned_hyphenation_3170
  run_exact 'ocr::processor::execution::dehyphenation::tests::joins_only_witnessed_aligned_page_width_wraps' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::joins_lexical_hyphen_wraps_without_dropping_the_dash' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::hard_hyphen_wraps_keep_their_dash' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::a_soft_hyphen_is_outside_the_scanned_wrap_policy' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::repairs_only_a_witnessed_terminal_word_within_the_same_block' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::sparse_lines_do_not_consume_an_entire_following_block' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::digits_and_non_ascii_edges_do_not_license_joins' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::sparse_lines_can_share_a_wrap_without_merging_their_elements' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::narrow_sparse_blocks_do_not_licence_a_multiword_wrap' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::joins_a_short_paragraph_final_continuation_after_a_full_line' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::uses_a_spelling_witness_with_unicode_surrounding_punctuation' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::joins_an_accented_wrap_with_an_independent_case_insensitive_witness' --lib
  run_exact 'ocr::processor::execution::dehyphenation::tests::keeps_an_accented_lexical_hyphen_when_both_spellings_are_witnessed' --lib
  run_exact 'ocr::processor::execution::dehyphenation::continuation_tests::sparse_terminal_wrap_joins_with_bracketed_body_geometry' --lib
  run_exact 'ocr::processor::execution::dehyphenation::continuation_tests::sparse_terminal_wrap_rejects_unproved_body_geometry' --lib
  run_exact 'ocr::processor::execution::dehyphenation::continuation_tests::sparse_terminal_wrap_stops_at_a_right_lane_overrun' --lib
  run_exact 'ocr::processor::execution::dehyphenation::continuation_tests::sparse_terminal_wrap_requires_spelling_and_unclaimed_ownership' --lib
  run_exact english_ocr_preserves_the_reporters_spanish_body --ignored --test issue_2951_spanish_layer
  run_exact correction_does_not_rewrite_the_same_spelling_at_a_disjoint_position --ignored --test issue_2951_spanish_layer
  run_exact recovery_does_not_publish_invisible_native_text --ignored --test issue_2951_spanish_layer
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::repeated_khmer_anchor_abstains_when_structured_intervening_words_disagree' --lib
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::same_height_columns_with_repeated_words_do_not_share_an_inline_anchor' --lib
  run_exact font_ink_proof_excludes_overlapping_annotation_appearances --native-pdf --test annotation_appearance_normal
  local single_title='structure::spatial_table_detector::source_headers::tests::single_title'
  expected=$((expected + 19))
  local single_title_case
  for single_title_case in \
    single_merged_title_promotes_bold_native_leaf_labels_without_changing_cells \
    single_merged_title_keeps_dark_rules_when_filled_dividers_are_painted_last \
    single_merged_title_requires_two_numeric_body_cells \
    single_merged_title_requires_empty_title_peers \
    single_merged_title_requires_the_upper_leaf_rule \
    single_merged_title_does_not_promote_regular_leaves \
    single_merged_title_refuses_a_divider_running_through_the_title \
    single_merged_title_requires_a_continuous_lower_leaf_rule \
    single_merged_title_refuses_a_divider_with_later_pale_or_dashed_stroke \
    single_merged_title_refuses_ink_that_stays_in_the_first_track \
    single_merged_title_refuses_dark_fill_extended_by_a_pale_stroke \
    single_merged_title_accepts_measured_upper_font_metric_overhang \
    single_merged_title_rejects_overhang_beyond_the_upper_metric_allowance \
    single_merged_title_rejects_small_overhang_on_each_other_edge \
    single_merged_title_allowance_does_not_change_the_legacy_grouped_proof \
    single_merged_title_requires_at_least_three_columns \
    single_merged_title_requires_a_title_declared_over_two_columns \
    single_merged_title_requires_alphabetic_title_and_leaves \
    single_merged_title_accepts_overhang_at_exactly_the_upper_metric_allowance; do
    run_exact "$single_title::$single_title_case" --native-pdf --lib
  done
  run_exact 'extractors::warnings::tests::nested_warning_scope_restores_the_outer_sink' --native-pdf --lib
  run_exact 'extractors::warnings::tests::unwinding_restores_the_outer_warning_scope' --native-pdf --lib
  run_exact 'extractors::warnings::tests::automatic_warning_outside_a_scope_is_not_retained' --native-pdf --lib
  run_exact explicit_global_warnings_round_trip_with_a_bounded_backlog --native-pdf --test extraction_api_regression
  run_exact shared_type0_font_replays_only_the_missing_tounicode_warning_once_per_document \
    --native-pdf --test extraction_api_regression
  run_exact concurrent_renderers_keep_their_own_dropped_glyph_warnings --native-pdf --test test_glyph_drop_reporting
  run_exact 'extractors::warnings::tests::overlapping_warning_scopes_keep_each_threads_warnings' --native-pdf --lib
  run_exact a_dropped_glyph_warns_on_every_page_it_paints --native-pdf --test test_glyph_drop_reporting
  run_exact 'rendering::text_rasterizer::tests::glyph_drop_warning_hides_font_and_preserves_details' --native-pdf --lib
  run_exact 'render_parity::fully_mapped_fonts_render_without_drop_warnings' --native-pdf --test test_decoder_characterization
  run_exact 'render_parity::subset_missing_glyph_raises_one_drop_warning' --native-pdf --test test_decoder_characterization
  local jpx_test
  for jpx_test in \
    a_dictionary_width_half_the_codestreams_extracts_at_the_codestreams_size \
    a_dictionary_width_double_the_codestreams_extracts_at_the_codestreams_size \
    a_dictionary_that_agrees_with_the_codestream_extracts_with_no_size_warning \
    an_indexed_image_extracts_at_the_codestreams_size \
    concurrent_documents_keep_their_own_jpx_size_warnings; do
    run_exact "$jpx_test" --native-pdf --test gh1900_jpx_dict_size_disagrees_with_codestream
  done
  expected=$((expected + 16))
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::missing_middle_khmer_repeat_uses_the_original_occurrence_position' --lib
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::located_english_word_keeps_its_place_between_khmer_occurrences' --lib
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::missing_khmer_occurrence_is_published_inline_in_both_surfaces' --lib
  run_exact 'extractors::pdf::ocr::layer_token_recovery::minority_khmer::partial_khmer_read_spends_one_of_two_identical_occurrences' --lib
  run_exact automatic_english_ocr_keeps_visible_minority_khmer_on_a_fragmented_page --test pdf_khmer_minority_2826
  run_exact automatic_english_ocr_keeps_clean_khmer_and_rejects_hidden_and_wrong_mapped_khmer --test pdf_khmer_minority_2826
  run_exact mixed_ocr_keeps_khmer_between_english_and_rejects_unrelated_ink --test pdf_khmer_minority_2826
  run_exact every_native_page_keeps_a_range_that_selects_its_text --test plain_page_ranges_select_page_text
  run_exact 'pdf::structure::paragraphs::tests::continuation_merge_distinguishes_zero_baseline_from_missing_geometry' --lib
  run_exact 'pdf::structure::adapters::tests::ocr_line_baseline_requires_a_source_box' --lib
  run_exact 'pdf::structure::pipeline::tests::native_paragraphs_retain_the_measured_zero_line_baseline' --lib
  run_exact 'pdf::structure::paragraphs::tests::display_rows_tests::continuation_merge_keeps_indexed_display_rows_with_changed_source_literals' --lib
  run_exact 'embedded_psm6_cache_tests::cached_default_recovers_embedded_line_in_cold_and_warm_reads' --lib
  run_exact accepted_tinted_table_keeps_the_header_and_first_record_separate \
    --test ocr_standalone_row_layout_2432
  expected=$((expected + 9))
  run_exact implicit_sparse_mode_certifies_tinted_leader_rows_in_plain_and_html --test ocr_sparse_table_html_2503
  run_exact explicit_sparse_and_single_block_modes_keep_the_stock_tinted_leader_transcript \
    --test ocr_sparse_table_html_2503
  run_exact html_columned_prose_stays_prose_and_json_remains_unassociated --test ocr_sparse_table_html_2503
  run_exact explicit_sparse_psm_plain_page_publishes_the_proven_table_rows --test ocr_sparse_table_page_rows
  run_exact accepted_table_keeps_rows_and_a_repeated_caption_in_plain_and_djot_pages \
    --test ocr_sparse_table_page_rows
  local sparse_proof
  for sparse_proof in \
    standalone_sparse_grid_preserves_one_non_numeric_value_below_first_row \
    standalone_value_exception_requires_exact_inventory_and_independent_tracks \
    merged_header_recovery_preserves_one_uncertain_literal_and_requires_disjoint_rows \
    merged_header_value_exception_cannot_move_to_a_later_record; do
    run_exact "ocr::processor::execution::full_scan_table::tests::$sparse_proof" --lib
  done
  run_exact tall_lower_density_plain_gray_flate_recovers_all_source_words_in_order --test pdf_tall_scan_strips
  run_exact png_predictor_panel_recovers_missing_line_without_changing_primary_ocr \
    --test pdf_embedded_psm6_addition_2327
  run_exact automatic_ocr_keeps_every_decodable_layer_token_of_a_page_with_an_undecodable_header --test ocr_layer_tokens_2952
  run_exact the_structured_markdown_page_keeps_the_same_layer_tokens_in_reading_order --test ocr_layer_tokens_2952
  run_exact the_table_of_contents_page_keeps_its_clean_read_without_the_layers_question_marks --test ocr_layer_tokens_toc_3027
  run_type0_cases
  run_type0_noisy_render_cases
  run_exact automatic_ocr_keeps_overprinted_separation_text_and_native_sibling --test pdf_separation_overprint_2817
  local type0_structure='extractors::pdf::ocr::native_word_coverage::structure_tests::type0_structure'
  run_exact "$type0_structure::single_page_type0_keeps_cell_only_tables_absent_from_the_structure" --lib
  local mixed_tables='extractors::pdf::ocr::tests::tests::mixed_cell_only_tables'
  run_exact "$mixed_tables::mixed_ocr_retains_cell_only_sidecar_tables_in_page_output" --lib
  run_budget_retry_cases
  run_retry_timeout_cases
  expected=$((expected + 10))
  local tall_scan_policy='extractors::pdf::ocr::tall_source::policy::tests'
  run_exact "$tall_scan_policy::reports_an_ocr_preprocessing_clamp_on_a_full_density_pdf_render" --lib
  run_exact "$tall_scan_policy::reports_the_actual_density_of_a_budgeted_pdf_read" --lib
  run_exact "$tall_scan_policy::strip_recovery_early_returns_report_only_the_proven_density_once" --lib
  run_exact 'extractors::pdf::ocr::tall_source::tests::bilevel_above_maximum_density_is_not_reported_as_below_minimum' --lib
  run_exact a_120_dpi_bilevel_scan_explains_the_source_density_refusal --test pdf_tall_scan_strips
  run_exact bilevel_and_jpeg_scans_rendered_at_120_dpi_keep_the_standard_floor_and_say_so --test pdf_tall_scan_strips
  run_exact tall_scans_below_the_density_floor_say_recovery_was_not_used --test pdf_tall_scan_strips
  run_exact fractional_bilevel_source_density_is_floored_below_the_minimum --test pdf_tall_scan_strips
  run_exact an_unsupported_tall_jpeg_reports_one_preprocessing_warning_cold_and_warm --test pdf_tall_scan_strips
  run_exact the_mixed_route_names_the_configuration_that_blocked_strip_recovery --test pdf_tall_scan_strips
}

run_retry_timeout_cases() {
  local module='extractors::pdf::ocr::tests::tests::retry_timeout_stop_tests'
  run_exact "$module::final_failed_page_keeps_its_warning_and_marks_the_run_cut_short" --lib
  run_exact "$module::final_page_that_fails_at_the_deadline_is_reported_as_unread" --lib
  run_exact "$module::explicit_ocr_cancellation_wins_at_the_deadline" --lib
  run_exact "$module::cancelled_token_wins_over_backend_error_at_the_deadline" --lib
  run_exact 'extractors::pdf::ocr::pipeline::in_flight_page_tests::queued_ocr_error_keeps_pre_deadline_classification' --lib
  run_exact 'extractors::pdf::ocr::pipeline::in_flight_page_tests::cancellation_interrupts_the_wait_for_a_stuck_page_during_soft_stop' --lib
  run_exact 'core::extractor::time_limit::tests::cancellation_at_the_deadline_takes_precedence_over_starting_grace' --lib
  run_exact 'core::extractor::time_limit::tests::cancellation_during_grace_takes_precedence_when_extraction_never_returns' --lib
  run_exact "$module::a_stop_that_skips_the_only_failed_pages_retry_returns_timeout" --lib
  run_exact "$module::a_stop_accounts_for_every_pending_failure_in_the_batch" --lib
}

run_budget_retry_cases() {
  expected=$((expected + 1))
  run_exact tall_gray_jpeg_recovers_every_source_word_after_render_budget_refusal --test pdf_tall_scan_strips
  local module='extractors::pdf::ocr::pipeline::direct_rgb_budget_recovery::tests'
  local test_name
  for test_name in \
    only_a_typed_source_budget_failure_is_eligible_for_the_direct_rgb_retry \
    retry_admission_rejects_each_nondefault_request \
    retry_admission_requires_the_registered_builtin_tesseract \
    rgb_certificate_rejects_a_source_one_byte_over_the_limit \
    rgb_certificate_requires_matching_encoded_and_placed_dimensions \
    rgb_certificate_accepts_both_pages_at_the_original_render_budget \
    an_uncertified_second_page_disqualifies_the_document_retry \
    rgb_certificate_rejects_a_stored_gray_jpeg \
    rgb_certificate_rejects_a_second_painted_image \
    two_image_reads_do_not_report_one_page_segmentation_mode \
    a_budget_failure_without_original_pdf_bytes_is_not_recovered; do
    run_exact "$module::$test_name" --lib
  done
  for test_name in \
    automatic_direct_rgb_budget_retry_retains_text_with_default_recognition_policy \
    explicitly_forced_minimal_direct_ocr_keeps_its_budget_refusal \
    layout_request_keeps_the_original_rgb_render_budget_refusal \
    minimal_direct_retry_does_not_omit_visible_paint_beside_the_rgb_scan \
    direct_rgb_budget_retry_retains_the_producing_tesseract_segmentation_mode \
    mixed_pipeline_retry_records_segmentation_mode_for_the_recovered_page; do
    run_exact "$test_name" --test pdf_direct_rgb_budget_recovery
  done
}

run_type0_cases() {
  local before="$passed"
  run_exact should_preserve_unrecovered_type0_page_when_first_in_mixed_document --test pdf_type0_scope_2533
  run_exact should_preserve_unrecovered_type0_page_when_second_in_mixed_document --test pdf_type0_scope_2533
  run_exact should_force_only_the_requested_page_in_either_document_order --test pdf_type0_scope_2533
  run_exact 'negative::should_choose_visible_outline_words_when_bad_marker_layer_is_first' --test pdf_type0_scope_2533
  run_exact 'negative::should_choose_visible_outline_words_when_bad_marker_layer_is_second' --test pdf_type0_scope_2533
  run_exact 'marker_probe::probe_read_replaces_the_partial_scan_and_keeps_no_marker_coverage_layer' --test pdf_type0_scope_2533
  if [ "$((passed - before))" -ne 6 ]; then
    printf 'Expected six executed public Type0 tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

# This suite builds without PaddleOCR, so the single-backend routes read the pages here. The full
# build always runs the OCR pipeline and never reaches them.
run_type0_noisy_render_cases() {
  local before="$passed"
  local test_name
  for test_name in \
    a_speckled_type0_layer_keeps_native_text_in_markdown \
    a_lightly_speckled_type0_layer_keeps_native_text_in_markdown \
    a_speckled_type0_layer_keeps_native_text_after_ink_loss \
    a_speckled_single_page_type0_layer_keeps_native_text_after_ink_loss \
    a_covered_type0_layer_sharing_a_few_words_with_the_visible_read_goes_to_ocr; do
    run_exact "$test_name" --test pdf_type0_covered_layer_2802
  done
  if [ "$((passed - before))" -ne 5 ]; then
    printf 'Expected five executed noisy-render Type0 tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_number_budget_cases() {
  local module='extractors::pdf::ocr::number_recovery::tests::run_controls_tests'
  run_exact "${module}::should_allow_supported_source_work_after_bounded_late_declines" --lib
  run_exact "${module}::should_exclude_unsupported_sources_from_eligible_page_limit" --lib
  run_exact "${module}::should_keep_unmeasured_shares_within_the_frozen_document_limit" --lib
  run_exact "${module}::should_enforce_source_reservation_as_an_allocation_ceiling" --lib
  run_exact "${module}::should_fail_closed_when_measured_source_work_exceeds_its_reservation" --lib
}

run_ink_probe_cases() {
  local before="$passed"
  local test_name
  for test_name in \
    custom_backend_reads_each_encoded_input_and_keeps_words_and_confidence \
    a_failed_probe_retains_native_text_and_does_not_reuse_a_failed_read \
    custom_backend_reads_do_not_survive_an_extraction \
    cancellation_after_the_probe_does_not_publish_its_accepted_read \
    probe_and_production_configs_can_return_different_source_words \
    a_changed_backend_model_is_read_again_after_the_accepted_probe \
    production_keeps_the_table_payload_of_its_configured_read \
    a_failed_custom_stage_and_its_fallback_run_on_each_encoded_input \
    a_quality_retry_still_runs_and_keeps_the_confident_fallback \
    custom_backend_keeps_the_table_payload_after_a_changed_encoding \
    changed_traineddata_is_not_reused_even_when_backend_version_is_unchanged; do
    run_exact "$test_name" --test pdf_ink_probe_reuse_2862
  done
  for test_name in \
    final_read_takes_the_probe_read_once_for_a_contradicted_page \
    finish_retains_only_contradicted_pages \
    same_pixels_on_another_page_are_not_reused \
    zero_retention_limit_reads_again_and_returns_each_document_intact \
    identity_changes_miss_without_consuming_the_matching_read \
    cancelled_read_returns_cancelled_without_ocr_or_consuming_the_read \
    plugin_reuse_bypass_does_not_consult_version \
    geometry_observer_bypasses_retained_reads \
    failed_backend_read_is_not_retained \
    model_change_during_a_read_is_not_retained \
    unkeyable_reads_are_never_retained \
    model_identity_tracks_override_traineddata_and_osd \
    stock_builtin_reuses_its_own_raw_read_once \
    stock_builtin_does_not_reuse_a_different_decode_budget \
    stock_builtin_does_not_reuse_a_different_encoded_working_size \
    custom_backend_reads_changed_encoding_even_when_rgb_pixels_match \
    model_change_alone_requires_a_fresh_backend_read \
    changed_policy_preserves_fresh_source_words_and_table_payload \
    model_contents_are_hashed_once_for_two_probe_reads_and_two_hits \
    tesseract_typed_plugin_that_is_not_the_builtin_reads_every_time \
    nested_skipped_payload_over_the_limit_is_not_retained \
    retention_cap_does_not_follow_a_huge_requested_content_limit \
    insert_clones_the_read_before_waiting_for_the_state_lock \
    recent_model_with_an_unchanged_stamp_is_rechecked_before_a_hit; do
    run_exact "extractors::pdf::ocr::read_reuse::tests::$test_name" --lib
  done
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::stock_raw_read_of_a_text_raster_matches_an_uncached_final_read' --lib
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::stock_public_mixed_route_reads_the_wrong_script_page_once' --lib
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::final_mixed_route_frees_a_retained_read_it_did_not_take' --lib
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::direct_mixed_route_reuses_the_contradicted_pages_probe_read' --lib
  if [ "$((passed - before))" -ne 39 ]; then
    printf 'Expected 39 executed ink-probe reuse tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_numeral_noise_cases() {
  local before="$passed"
  run_exact the_ocr_result_counts_a_line_kept_for_its_numeral_apart_from_a_removed_one \
    --test ocr_numeral_noise_count_2851
  run_exact a_rejected_table_region_gets_its_filtered_words_back_through_real_ocr \
    --test ocr_numeral_noise_count_2851
  run_exact 'ocr::processor::execution::tests::restore_rejected_table_words_records_the_kept_numeral_line_count_of_the_new_parse' --lib
  run_exact 'ocr::tesseract_backend::tests::warnings_from_ocr_metadata_reports_only_the_removed_lines' --lib
  run_exact 'ocr::tesseract_backend::tests::strip_ocr_scratch_metadata_keys_preserves_genuine_user_metadata' --lib
  run_exact 'ocr::tesseract_backend::tests::process_image_file_reports_the_removed_line_and_keeps_both_counts_out_of_the_metadata' --lib
  run_exact 'ocr::tesseract_backend::tests::the_backend_content_holds_the_words_won_back_for_a_rejected_table_region' --lib
  if [ "$((passed - before))" -ne 7 ]; then
    printf 'Expected 7 executed numeral-noise tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_retained_read_cases() {
  local before="$passed"
  local module='extractors::pdf::ocr::rendering::retained_read_metadata_tests'
  local test_name
  for test_name in \
    rejected_first_read_does_not_supply_retained_page_preprocessing \
    a_rejected_single_read_sets_neither_preprocessing_nor_tesseract_mode \
    a_retained_single_read_keeps_its_preprocessing_and_tesseract_mode \
    pooled_rejections_do_not_supply_page_preprocessing \
    small_legible_read_survives_garbage_in_the_same_pool \
    counts_two_character_fragments_regardless_of_utf8_width \
    tiny_garbage_reads_are_still_rejected_when_their_pool_has_enough_words \
    a_legible_small_read_keeps_its_table_and_formula_when_its_pool_is_noisy \
    only_reads_with_text_or_structured_content_supply_page_metadata \
    zero_fragmentation_threshold_still_rejects_a_numeric_only_pool \
    unpublished_preprocessing_does_not_consume_the_output_budget; do
    run_exact "${module}::$test_name" --lib
  done
  if [ "$((passed - before))" -ne 11 ]; then
    printf 'Expected 11 executed retained-read tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_discarded_recovery_cases() {
  local before="$passed"
  local test_name
  for test_name in \
    discarded_noisy_image_retry_has_no_recovery_claim \
    retained_noisy_image_retry_keeps_its_recovery_claim \
    discarded_noisy_pipeline_retry_has_no_recovery_claim \
    retained_noisy_pipeline_retry_keeps_its_recovery_claim \
    discarded_noisy_whole_document_retry_has_no_recovery_claim \
    retained_noisy_whole_document_retry_keeps_its_recovery_claim \
    discarded_noisy_retry_reports_native_text_kept \
    discarded_noisy_pipeline_retry_reports_native_text_kept \
    discarded_noisy_all_pages_pipeline_retry_reports_native_text_kept; do
    run_exact "discarded_noise_2908::$test_name" --test ocr_page_failures
  done
  run_exact a_directly_called_backend_records_the_page_recovered_as_a_table --test ocr_page_failures
  run_exact a_whole_document_run_records_every_failed_page --test ocr_page_failures
  if [ "$((passed - before))" -ne 11 ]; then
    printf 'Expected 11 executed embedded-image recovery warning tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_office_page_cases() {
  local test_name
  run_exact should_preserve_source_spacing_across_inline_rtf_groups --test rtf_extractor_tests
  for test_name in \
    plain_heading_carries_no_style_note \
    plain_table_cell_keeps_link_text_and_drops_the_target \
    markdown_table_cell_keeps_its_link \
    plain_image_carries_no_placeholder \
    plain_image_keeps_its_description \
    plain_table_cell_keeps_its_math_delimiters; do
    run_exact "$test_name" --test docx_plain_content
  done
  for test_name in \
    every_page_of_a_docx_memo_keeps_a_range_that_selects_its_page_text \
    each_page_of_a_two_page_docx_keeps_a_range_that_selects_its_page_text \
    each_slide_of_a_pptx_keeps_a_range_that_selects_its_page_text; do
    run_exact "$test_name" --test plain_page_ranges_select_page_text
  done
  for test_name in \
    should_keep_an_equation_apart_from_a_following_letter_in_plain_page_content \
    should_number_page_content_notes_like_the_rendered_output \
    should_keep_a_footnote_and_an_endnote_with_the_same_word_id_apart_in_page_content \
    should_keep_footnote_definitions_before_consecutive_trailing_page_breaks \
    plain_table_cell_carries_no_footnote_marker \
    should_write_the_description_of_a_drawn_shape_in_its_place \
    should_write_no_placeholder_for_a_picture_with_an_empty_description; do
    run_exact "extractors::docx::tests::$test_name" --lib
  done
  run_exact 'extractors::docx::tests::direct_docx_reads_from_the_only_foreign_blocking_worker_complete' --lib
  run_exact 'extractors::docx::tests::should_parse_inline_without_tokio_and_check_cancellation_before_parsing' --lib
  run_exact 'extractors::docx::tests::should_resume_a_blocking_docx_worker_panic' --lib
  run_exact 'extractors::docx::tests::should_report_docx_worker_shutdown_as_cancellation' --lib
  run_exact 'extractors::docx::tests::should_match_single_extraction_in_batch_mode' --lib
  run_exact 'extractors::excel::tests::a_plain_sheet_page_starts_with_its_sheet_name' --lib
  run_exact 'extractors::excel::tests::excel_batch_runs_bytes_and_paths_on_a_blocking_worker' --lib
  run_exact 'extractors::excel::tests::single_excel_reads_queue_and_return_typed_cancellation' --lib
  run_exact 'extractors::excel::tests::direct_excel_reads_from_the_only_foreign_blocking_worker_complete' --lib
  run_exact 'extractors::excel::tests::single_and_batch_excel_outputs_match_for_bytes_and_paths' --lib
  run_exact 'extractors::excel::tests::cancelled_excel_worker_does_not_call_the_parser' --lib
  run_exact 'extractors::excel::tests::single_excel_read_resumes_the_worker_panic' --lib
  run_exact 'extractors::excel::tests::pre_cancelled_excel_path_returns_cancellation_inline' --lib
  run_exact 'extractors::docx::tests::docx_queues_on_a_shared_worker_and_honors_typed_cancellation' --lib
  run_exact 'engine::extract_impl::tests::a_panicking_excel_read_leaves_sibling_batch_results_intact' --lib
  run_exact 'core::runtime::large_stack_tests::blocking_work_reports_cancellation_after_the_parser_returns' --lib
  run_exact 'core::runtime::large_stack_tests::dropping_a_queued_blocking_worker_prevents_parser_entry' --lib
  run_exact 'core::runtime::large_stack_tests::blocking_work_keeps_the_callers_thread_local_dispatcher_and_span' --lib
  run_exact 'rendering::plain::tests::render_plain_leaves_out_the_docx_style_name_of_a_heading' --lib
  local module='extraction::derive::pages::plain_page_rerender_tests'
  for test_name in \
    a_docx_page_with_its_own_table_markdown_gets_its_part_of_the_plain_content \
    every_page_of_a_docx_document_with_ocr_pages_gets_its_part_of_the_plain_content \
    a_markdown_docx_page_keeps_the_range_its_extractor_measured \
    a_docx_range_measured_on_the_parser_text_selects_its_page_text_in_the_plain_content \
    pages_with_the_same_text_each_get_the_range_of_their_own_text \
    a_page_with_text_and_no_elements_keeps_its_text_and_loses_its_range_with_a_warning \
    a_page_with_no_text_loses_its_range_without_a_warning \
    a_page_whose_elements_write_only_whitespace_loses_its_range_without_a_warning \
    a_page_keeps_its_text_when_no_element_of_the_document_writes_text \
    the_last_page_with_text_keeps_its_range_when_a_later_page_writes_only_whitespace \
    a_plain_page_has_the_plain_text_of_its_elements_with_the_note_numbers_of_the_document \
    a_prebuilt_docx_page_has_its_part_of_the_plain_content_and_keeps_its_notes \
    a_prebuilt_pdf_page_keeps_its_text \
    a_note_marker_after_the_last_text_of_a_page_stays_on_that_page \
    the_range_of_a_page_number_with_no_extracted_page_is_omitted_with_a_warning \
    the_warning_names_a_page_with_two_omitted_ranges_one_time \
    the_warning_names_omitted_pages_in_page_order_and_no_range_is_left; do
    run_exact "${module}::$test_name" --lib
  done
}

run_mixed_plain_cases() {
  local before="$passed"
  local test_name
  for test_name in \
    mixed_plain_keeps_section_labels_the_transcript_lacks_on_a_three_table_page \
    mixed_plain_keeps_a_label_before_emphasis_the_transcript_lacks \
    mixed_plain_declines_elements_that_add_words_to_the_accepted_table \
    mixed_plain_admits_rendered_inline_prose_beside_a_table; do
    run_exact "extractors::pdf::tests::$test_name" --lib
  done
  for test_name in \
    detected_scan_without_native_elements_renders_the_accepted_table \
    selected_scan_renders_prose_before_and_after_the_ocr_table \
    selected_scan_keeps_section_labels_that_table_assembly_drops; do
    run_exact "mixed_tables::$test_name" --test pdf_all_pages_native_preservation_2768
  done
  run_exact 'extractors::pdf::ocr::pipeline::mixed_plain::inline_group_tests::mixed_plain_drops_a_repeat_of_projected_prose' --lib
  if [ "$((passed - before))" -ne 8 ]; then
    printf 'Expected 8 executed mixed Plain tests, observed %s\n' "$((passed - before))" >&2
    exit 1
  fi
}

run_cancellation_cases() {
  local thresholds='extractors::image::image_color_threshold::tests'
  local cache='core::extractor::cache_context'
  run_exact 'cancellation::tests::check_cancelled_only_rejects_a_cancelled_token' --lib
  run_exact "$thresholds::cancellation_before_the_probe_prevents_all_retry_work" --lib
  run_exact "$thresholds::cancellation_during_the_probe_prevents_ocr" --lib
  run_exact "$thresholds::cancellation_during_ocr_discards_the_candidate" --lib
  run_exact "$cache::tests::cancellation_precedes_model_lookup_and_invalidates_cache_work" --lib
  run_exact "$cache::explicit_mode_tests::outer_cache_rejects_legacy_retry_entry_after_model_removal" --lib
  run_exact "$cache::explicit_mode_tests::image_osd_identity_observes_equal_length_revisions_without_pdf_support" --lib
  run_exact "$cache::explicit_mode_tests::non_osd_image_modes_keep_empty_context_without_model_io" --lib
  run_exact "$cache::explicit_mode_tests::explicit_osd_modes_apply_to_pdf_and_images_but_not_other_backends" --lib
  run_exact 'extractors::pdf::ocr::orientation_retry::tests::recognition_cancellation_and_exhaustion_precede_the_cache_probe' --lib
  run_exact 'extractors::pdf::tests::native_pdf_blocking_phase_stops_when_cancelled' --lib
}

run_api_cases() {
  run_exact 'api::handlers::tests::test_extract_async_then_poll_job_id' --lib
  run_exact 'api::jobs::tests::test_cancel_running_job_fires_token' --lib
  run_exact 'api::jobs::tests::test_complete_after_cancel_is_noop' --lib
  run_exact 'engine::tests::single_and_batch_extraction_ignore_pre_version_cache_entries_and_reuse_current_ones' --lib
}

case "$suite" in
pdf-ocr)
  expected=139
  check_mixed_decision_layer_handoffs
  run_pdf_cases
  run_retained_read_cases
  expected=$((expected + 11 + 7))
  run_number_budget_cases
  expected=$((expected + 1))
  run_ink_probe_cases
  run_discarded_recovery_cases
  run_exact 'extractors::pdf::ocr::tests::tests::should_error_when_every_page_fails_and_nothing_can_be_recovered' --lib
  run_exact 'core::extractor::file::issue_217_fallback_tests::pdf_ocr_failure_allows_extractor_fallback_but_backend_cancellation_is_terminal' --lib
  run_exact 'extractors::pdf::tests::automatic_ocr_failure_with_no_native_text_returns_an_error' --lib
  run_exact 'extractors::pdf::ocr::failed_page_record_tests::a_pipeline_whose_stages_all_fail_carries_every_page_record' --lib
  run_exact 'extractors::pdf::ocr::page_failure_cause_tests::all_pages_failing_keeps_the_cause_chain_everywhere' --lib
  run_exact 'extractors::pdf::ocr::page_failure_cause_tests::pdf_ocr_trace_keeps_error_categories_without_backend_text' --lib
  run_exact 'extractors::pdf::ocr::pipeline::tracing_tests::public_mixed_pdf_fallback_warns_without_exposing_the_backend_error' --lib
  run_exact 'extractors::pdf::ocr::pipeline::tracing_tests::public_inline_image_ocr_warns_without_exposing_the_backend_error' --lib
  run_exact 'error::tests::cli_diagnostic_omits_raw_pdf_ocr_sources_from_debug' --lib
  run_exact 'extractors::pdf::tests::inline_image_ocr_backend_cancellation_stops_pdf_extraction' --lib
  run_exact 'extractors::pdf::tests::test_ocr_inline_images_uses_configured_backend' --lib
  run_exact 'extractors::pdf::tests::test_ocr_inline_images_forwards_output_format' --lib
  run_exact a_targeted_retry_backend_cancellation_discards_the_successful_whole_read --test pdf_marked_page_retry_fallback_2833
  run_exact 'commands::extract::ocr_diagnostic_tests::cli_pdf_ocr_failure_omits_backend_diagnostics_in_both_formats' --cli
  run_exact 'commands::extract::tests::uri_extraction_failure_does_not_assert_a_false_readability_diagnosis' --cli
  expected=$((expected + 15))
  run_numeral_noise_cases
  expected=$((expected + 8))
  run_mixed_plain_cases
  expected=$((expected + 11))
  run_cancellation_cases
  run_exact psm6_html_preserves_scan_rows_only_with_owned_columns --test pdf_auto_dot_leader_rows_2503
  run_exact psm6_html_leaves_each_page_of_a_two_page_scan_flattened --test pdf_auto_dot_leader_rows_2503
  run_exact psm6_html_keeps_document_structure_flattened --test pdf_auto_dot_leader_rows_2503
  run_exact psm6_html_keeps_a_detached_scan_flattened_beside_native_text --test pdf_auto_dot_leader_rows_2503
  run_exact psm6_html_keeps_element_output_flattened --test pdf_auto_dot_leader_rows_2503
  run_exact psm6_markdown_keeps_scan_rows_flattened --test pdf_auto_dot_leader_rows_2503
  run_exact 'extractors::pdf::ocr::document::tests::certified_scan_html_rows_require_each_admission_condition' --lib
  run_exact 'rendering::comrak_bridge::tests::marked_paragraph_rows_escape_html_text_in_each_row' --lib
  run_exact six_column_dot_leader_scan_keeps_every_row_in_html --test ocr_six_column_html_2503
  run_exact six_column_dot_leader_picture_keeps_every_row_in_html --test ocr_six_column_html_2503
  run_exact no_leader_picture_keeps_its_rows_in_html --test ocr_six_column_html_2503
  run_exact nine_pixel_tinted_scan_keeps_every_source_row --test ocr_low_ppi_scan_replay_3337
  run_exact seven_pixel_tinted_scan_keeps_its_measured_baseline --test ocr_low_ppi_scan_replay_3337
  run_exact low_ppi_table_rows_keep_their_measured_floors --test ocr_ppi_rows_2503
  expected=$((expected + 14))
  ;;
pdf-sparse-paddle)
  expected=16
  for name in \
    automatic_ocr_keeps_overprinted_separation_text_and_native_sibling \
    reporter_folio_keeps_the_exact_transcript_with_automatic_ocr \
    mixed_size_type0_text_keeps_the_exact_transcript_at_normal_density; do
    run_exact "$name" --test pdf_separation_overprint_2817
  done
  sparse_module="extractors::pdf::ocr::pipeline::sparse_glyph_read"
  for name in \
    tests::a_marked_word_requires_a_letter_or_digit_at_each_missing_position \
    page_quality_rejection_retries_sparse_glyphs_with_normal_raster_and_preprocessing \
    missing_marked_word_retries_sparse_glyphs_with_normal_raster_and_preprocessing \
    active_pdf_margin_uses_one_normal_density_read \
    implicit_pipeline_retries_missing_marked_words_at_normal_density \
    implicit_pipeline_preserves_normal_render_on_rotated_or_failed_trials \
    implicit_trial_renderer_uses_the_resolved_page_rotation \
    implicit_trial_skips_pages_that_khmer_number_recovery_may_claim \
    review_tests::implicit_tesseract_paddle_pipeline_reads_at_trial_density \
    review_tests::explicit_tesseract_paddle_pipeline_keeps_the_normal_raster \
    review_tests::explicit_vlm_fallback_keeps_the_normal_raster \
    review_tests::explicit_paddle_backend_receives_the_normal_raster \
    review_tests::sparse_preview_preserves_the_detached_recovery_context; do
    run_exact "$sparse_module::$name" --lib
  done
  ;;
office-pages)
  expected=53
  run_office_page_cases
  ;;
api-jobs)
  expected=4
  run_api_cases
  ;;
pdf-table-paint)
  expected=24
  table_module="pdf::structure::regions::table_recognition"
  run_exact "$table_module::structure::row_span_closure::tests::closed_native_rowspan_preserves_heading_and_rejects_incomplete_evidence" --lib
  run_exact "$table_module::structure::column_spans::tests::split_rowspan_retains_only_column_spans_with_open_source_rules" --lib
  run_exact "$table_module::structure::column_spans::tests::split_header_rejects_a_displaced_rule_between_native_words" --lib
  for name in \
    native_tatr_vertical_span_stops_at_a_painted_rule \
    native_tatr_vertical_span_survives_a_rule_outside_the_merged_track \
    native_tatr_three_row_span_retains_only_the_source_supported_merge \
    native_tatr_preserves_adjacent_source_supported_rowspans \
    native_tatr_preserves_three_adjacent_rowspans_across_empty_continuations \
    native_tatr_preserves_a_tinted_open_track \
    native_tatr_resolves_open_and_divided_tracks_on_full_width_tints \
    native_tatr_requires_continuous_flank_rule_on_a_tint \
    native_tatr_preserves_a_noisy_near_white_open_track \
    native_tatr_rowspan_resumes_after_a_track_divider_with_a_left_neighbor; do
    run_exact "$table_module::metadata_tests::$name" --lib
  done
  run_exact "$table_module::adjacent_span_tests::native_tatr_preserves_adjacent_spans_with_vertically_aligned_labels" --lib
  run_exact "$table_module::adjacent_span_tests::native_tatr_adjacent_span_stops_expanding_at_a_contradicted_neighbor" --lib
  run_exact "$table_module::adjacent_span_tests::native_tatr_adjacent_opening_accepts_a_shorter_neighbor" --lib
  run_exact "$table_module::adjacent_span_tests::native_tatr_adjacent_opening_checks_only_the_neighbor_prefix_at_the_boundary" --lib
  for name in \
    narrow_track_glyph_ink_does_not_count_as_a_divider \
    staggered_flank_rules_do_not_prove_an_opening \
    masked_divider_does_not_prove_an_opening \
    glyph_ink_in_either_flank_does_not_prove_an_opening \
    near_white_opening_requires_visible_band_columns \
    near_white_opening_checks_word_masks_on_rule_shoulders \
    aligned_flank_rules_require_at_least_eighteen_of_twenty_visible_columns; do
    run_exact "$table_module::structure::tests::$name" --lib
  done
  ;;
pdf-regression-pages)
  # ~keep The snapshots are written with the default features on, as the test's TEST_COMMAND
  # names, so this suite runs the whole test file rather than run_exact's --no-default-features.
  expected=24
  cargo_args=(test --locked -p xberg --features "$features" --test pdf_regression_pages)
  cargo "${cargo_args[@]}" -- --list --color never 2>&1 | tee "$log_dir/list.log"
  if ! grep -Fxq "$expected tests, 0 benchmarks" "$log_dir/list.log"; then
    printf 'Expected exactly %s collected regression page tests\n' "$expected" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" -- --color never 2>&1 | tee "$log_dir/run.log"
  if ! grep -Fq "test result: ok. $expected passed; 0 failed; 0 ignored;" "$log_dir/run.log"; then
    printf 'Expected exactly %s executed regression page tests\n' "$expected" >&2
    exit 1
  fi
  passed=$((passed + expected))
  ;;
esac
if [ "$passed" -ne "$expected" ]; then
  printf 'Expected %s executed focused tests, observed %s\n' "$expected" "$passed" >&2
  exit 1
fi
printf 'Focused %s suite passed %s exact tests\n' "$suite" "$passed"
