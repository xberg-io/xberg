#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir/../../.."

mode="${1:-all}"
case "$mode" in
all | synthetic) ;;
*)
  echo "Expected tall-scan mode: all or synthetic" >&2
  exit 1
  ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export HF_HUB_OFFLINE=1
export XBERG_CACHE_DIR="$tmp/xberg-cache"
export XBERG_TESSDATA_SYSTEM_PATHS=0
unset TESSDATA_PREFIX
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
export XBERG_2502_EVIDENCE="$tmp"
features="full,formula-recognition,classification-onnx,bundle-tessdata-eng"

check_guide() {
  local guide="$1" actual
  if [[ ! -f "$guide" ]]; then
    echo "Missing $guide. Fetch the pinned pdf corpus before running public tall-scan tests." >&2
    exit 1
  fi
  actual="$(shasum -a 256 <"$guide" | awk '{ print $1 }')"
  if [[ $actual != 04c76db01e20d7641e886b2403bea2df5757dfe9b4be7a283ee44d1861081a06 ]]; then
    echo "Public guide SHA-256 differs from the pinned test corpus" >&2
    exit 1
  fi
}

run_group() {
  local binary="$1" expected="$2" collected
  shift 2
  local cargo_args=(test --locked -p xberg --features "$features" --test "$binary")
  cargo "${cargo_args[@]}" -- "$@" --list --color never 2>&1 | tee "$tmp/list"
  collected="$(awk '/: test$/ { count++ } END { print count + 0 }' "$tmp/list")"
  if [[ $collected -ne $expected ]]; then
    echo "$binary: expected $expected collected tests, got $collected" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" -- "$@" --test-threads=1 --color never 2>&1 | tee "$tmp/run"
  if ! grep -qE "^test result: ok\. $expected passed; 0 failed; 0 ignored;" "$tmp/run"; then
    echo "$binary: expected $expected passing tests with none ignored" >&2
    exit 1
  fi
}

if [[ $mode == all ]]; then
  export XBERG_2502_GUIDE="$PWD/test_documents/pdf/sharable_web_guide.pdf"
  check_guide "$XBERG_2502_GUIDE"
fi

run_group pdf_tall_scan_strips 34
run_group pdf_tall_scan_generalization 4
if [[ $mode == all ]]; then
  run_group pdf_original_2502 1 public_original_multipage_recovery --exact --ignored
  run_group pdf_original_2502 1 public_page_eight_heading_keeps_structured_output_exact --exact --ignored
  run_group pdf_original_2502 1 public_page_five_undersampled_line_keeps_competing_owner --exact --ignored
  run_group pdf_original_2502 1 public_page_five_line_keeps_structured_output_exact --exact --ignored
  run_group pdf_tall_right_column_2426 5
  run_group pdf_tall_table_annotations_2502 1 guide_tables_and_real_annotations_survive_paired_reads --exact --ignored
fi
