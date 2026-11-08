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
}

run_api_cases() {
  run_exact 'api::handlers::tests::test_extract_async_then_poll_job_id' --lib
  run_exact 'api::jobs::tests::test_cancel_running_job_fires_token' --lib
  run_exact 'api::jobs::tests::test_complete_after_cancel_is_noop' --lib
  run_exact 'engine::tests::single_and_batch_extraction_ignore_pre_version_cache_entries_and_reuse_current_ones' --lib
}

case "$suite" in
pdf-ocr) run_pdf_cases ;;
api-jobs) run_api_cases ;;
esac
if [ "$passed" -lt 4 ]; then
  printf 'Expected at least four executed focused tests, observed %s\n' "$passed" >&2
  exit 1
fi
printf 'Focused %s suite passed %s exact tests\n' "$suite" "$passed"
