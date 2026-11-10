#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
suite="${1:?expected pdf-ocr or api-jobs}"
case "$suite" in
pdf-ocr) features="pdf,ocr,tokio-runtime,bundle-tessdata-eng" ;;
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
    cancellation_during_a_hit_returns_cancelled_without_ocr \
    geometry_observer_bypasses_retained_reads \
    failed_backend_read_is_not_retained \
    backend_version_change_during_a_read_is_not_retained \
    unkeyable_reads_are_never_retained \
    model_identity_tracks_override_traineddata_and_osd \
    stock_builtin_reuses_its_own_raw_read_once \
    stock_builtin_does_not_reuse_a_different_decode_budget \
    stock_builtin_does_not_reuse_a_different_encoded_working_size \
    custom_backend_reads_changed_encoding_even_when_rgb_pixels_match \
    model_change_alone_requires_a_fresh_backend_read \
    changed_policy_preserves_fresh_source_words_and_table_payload; do
    run_exact "extractors::pdf::ocr::read_reuse::tests::$test_name" --lib
  done
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::stock_raw_read_of_a_text_raster_matches_an_uncached_final_read' --lib
  run_exact 'extractors::pdf::ocr::read_reuse::stock_tests::stock_public_mixed_route_reads_the_wrong_script_page_once' --lib
  if [ "$((passed - before))" -ne 31 ]; then
    printf 'Expected 31 executed ink-probe reuse tests, observed %s\n' "$((passed - before))" >&2
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
  expected=41
  run_pdf_cases
  run_ink_probe_cases
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
