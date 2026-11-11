#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
suite="${1:?expected pdf-ocr or api-jobs}"
case "$suite" in
pdf-ocr) features="pdf,ocr,tokio-runtime,bundle-tessdata-eng,layout-types" ;;
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
  run_exact 'pdf::structure::paragraphs::tests::display_rows_tests::continuation_merge_keeps_indexed_display_rows_with_changed_source_literals' --lib
  run_exact cached_default_recovers_embedded_line_in_cold_and_warm_reads \
    --test pdf_embedded_psm6_addition_2327
  run_exact accepted_tinted_table_keeps_the_header_and_first_record_separate \
    --test ocr_standalone_row_layout_2432
  run_exact tall_lower_density_plain_gray_flate_recovers_all_source_words_in_order --test pdf_tall_scan_strips
  run_exact png_predictor_panel_recovers_missing_line_without_changing_primary_ocr \
    --test pdf_embedded_psm6_addition_2327
  run_type0_cases
  run_budget_retry_cases
}

run_budget_retry_cases() {
  run_exact tall_gray_jpeg_recovers_every_source_word_after_render_budget_refusal --test pdf_tall_scan_strips
  local module='extractors::pdf::ocr::pipeline::direct_rgb_budget_recovery::tests'
  local test_name
  for test_name in \
    only_a_typed_source_budget_failure_is_eligible_for_the_direct_rgb_retry \
    retry_admission_rejects_each_nondefault_request \
    retry_admission_requires_the_registered_builtin_tesseract \
    rgb_certificate_rejects_a_source_one_byte_over_the_limit \
    rgb_certificate_requires_matching_encoded_and_placed_dimensions \
    a_second_physical_page_disqualifies_an_otherwise_certified_rgb_source \
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
  if [ "$((passed - before))" -ne 5 ]; then
    printf 'Expected five executed public Type0 tests, observed %s\n' "$((passed - before))" >&2
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

run_api_cases() {
  run_exact 'api::handlers::tests::test_extract_async_then_poll_job_id' --lib
  run_exact 'api::jobs::tests::test_cancel_running_job_fires_token' --lib
  run_exact 'api::jobs::tests::test_complete_after_cancel_is_noop' --lib
  run_exact 'engine::tests::single_and_batch_extraction_ignore_pre_version_cache_entries_and_reuse_current_ones' --lib
}

case "$suite" in
pdf-ocr)
  expected=81
  run_pdf_cases
  run_retained_read_cases
  expected=$((expected + 11))
  run_number_budget_cases
  run_ink_probe_cases
  run_discarded_recovery_cases
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
