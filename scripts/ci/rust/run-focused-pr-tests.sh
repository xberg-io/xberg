#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
suite="${1:?expected pdf-ocr, office-pages or api-jobs}"
case "$suite" in
pdf-ocr) features="pdf,ocr,tokio-runtime,bundle-tessdata-eng,layout-types" ;;
office-pages) features="office,excel,tokio-runtime" ;;
api-jobs) features="api,tokio-runtime" ;;
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
  local -a cargo_args=(test --locked -p xberg --no-default-features --features "$features" "$@")
  cargo "${cargo_args[@]}" -- --exact "$test_name" --list --color never 2>&1 | tee "$log_dir/list.log"
  if ! grep -Fxq "$test_name: test" "$log_dir/list.log" ||
    ! grep -Fxq '1 test, 0 benchmarks' "$log_dir/list.log"; then
    printf 'Expected exactly one collected test: %s\n' "$test_name" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" -- --exact "$test_name" --color never 2>&1 | tee "$log_dir/run.log"
  if ! grep -Fxq "test $test_name ... ok" "$log_dir/run.log" ||
    ! grep -Fq 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log_dir/run.log"; then
    printf 'Expected exactly one executed test: %s\n' "$test_name" >&2
    exit 1
  fi
  passed=$((passed + 1))
}

run_pdf_cases() {
  run_exact every_native_page_keeps_a_range_that_selects_its_text --test plain_page_ranges_select_page_text
  run_exact 'pdf::structure::paragraphs::tests::display_rows_tests::continuation_merge_keeps_indexed_display_rows_with_changed_source_literals' --lib
  run_exact cached_default_recovers_embedded_line_in_cold_and_warm_reads \
    --test pdf_embedded_psm6_addition_2327
  run_exact accepted_tinted_table_keeps_the_header_and_first_record_separate \
    --test ocr_standalone_row_layout_2432
  run_exact tall_lower_density_plain_gray_flate_recovers_all_source_words_in_order --test pdf_tall_scan_strips
  run_exact png_predictor_panel_recovers_missing_line_without_changing_primary_ocr \
    --test pdf_embedded_psm6_addition_2327
  run_type0_cases
  run_type0_noisy_render_cases
  local type0_structure='extractors::pdf::ocr::native_word_coverage::structure_tests::type0_structure'
  run_exact "$type0_structure::single_page_type0_keeps_cell_only_tables_absent_from_the_structure" --lib
  local mixed_tables='extractors::pdf::ocr::tests::tests::mixed_cell_only_tables'
  run_exact "$mixed_tables::mixed_ocr_retains_cell_only_sidecar_tables_in_page_output" --lib
  run_budget_retry_cases
  run_retry_timeout_cases
}

run_retry_timeout_cases() {
  local module='extractors::pdf::ocr::tests::tests::retry_timeout_stop_tests'
  run_exact "$module::final_failed_page_keeps_its_warning_and_marks_the_run_cut_short" --lib
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
  if [ "$((passed - before))" -ne 38 ]; then
    printf 'Expected 38 executed ink-probe reuse tests, observed %s\n' "$((passed - before))" >&2
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
    discarded_noisy_all_pages_pipeline_retry_has_no_recovery_claim; do
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
  run_exact 'extractors::excel::tests::a_plain_sheet_page_starts_with_its_sheet_name' --lib
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

run_api_cases() {
  run_exact 'api::handlers::tests::test_extract_async_then_poll_job_id' --lib
  run_exact 'api::jobs::tests::test_cancel_running_job_fires_token' --lib
  run_exact 'api::jobs::tests::test_complete_after_cancel_is_noop' --lib
  run_exact 'engine::tests::single_and_batch_extraction_ignore_pre_version_cache_entries_and_reuse_current_ones' --lib
}

case "$suite" in
pdf-ocr)
  expected=93
  run_pdf_cases
  run_retained_read_cases
  expected=$((expected + 11 + 7))
  run_number_budget_cases
  run_ink_probe_cases
  run_discarded_recovery_cases
  run_numeral_noise_cases
  ;;
office-pages)
  expected=35
  run_office_page_cases
  ;;
api-jobs)
  expected=4
  run_api_cases
  ;;
esac
if [ "$passed" -ne "$expected" ]; then
  printf 'Expected %s executed focused tests, observed %s\n' "$expected" "$passed" >&2
  exit 1
fi
printf 'Focused %s suite passed %s exact tests\n' "$suite" "$passed"
