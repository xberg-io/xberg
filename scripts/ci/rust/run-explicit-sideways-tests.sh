#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"
[ -f "$REPO_ROOT/Cargo.toml" ] || {
  echo "Missing repository Cargo.toml" >&2
  exit 1
}
cd "$REPO_ROOT"

profile="${1:?Use the warm or native qualification profile}"
features="pdf,pdf-native,ocr,ocr-pipeline,tokio-runtime,bundle-tessdata-eng,language-detection,html,image-encode"
filters=()
expected_names=()
case "$profile" in
warm)
  features+=",auto-rotate-tract"
  expected=15
  expected_names=(
    caller_psm1_word_elements_on_an_upside_down_page_carry_no_internal_marker
    cached_context_changed_during_extraction_is_not_written
    cached_engine_bytes_recover_when_osd_is_installed_without_changing_the_configuration
    cached_file_recovers_when_osd_is_installed_without_changing_the_configuration
    prose::explicit_language_block_abstains_on_extra_prose_regions
    prose::explicit_language_block_recovers_the_sideways_source_paragraph
    explicit_language_block_recovers_the_counterclockwise_source_grid
    explicit_language_block_recovers_the_upside_down_source_grid
    explicit_language_block_retains_the_sideways_source_table
    explicit_osd_retains_the_original_source_grid_without_rotating_pixels
    helpers::tests::test_get_test_documents_dir
    helpers::tests::test_test_documents_available
    language_only_without_cached_osd_keeps_the_original_read
    omitted_and_false_rotation_are_the_same_typed_public_configuration
    upright_osd_preserves_its_existing_cells_and_content
  )
  if [ -z "${XBERG_ORIENTATION_TEST_MODEL:-}" ]; then
    repository=models--xberg-io--paddleocr-onnx-models
    revision=bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8
    artifact=v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx
    snapshot="${HF_HUB_CACHE:?Pinned model cache required}/${repository}/snapshots/${revision}"
    export XBERG_ORIENTATION_TEST_MODEL="${snapshot}/${artifact}"
  fi
  ;;
native)
  expected=9
  expected_names=(
    caller_psm1_word_elements_on_an_upside_down_page_carry_no_internal_marker
    cached_context_changed_during_extraction_is_not_written
    cached_engine_bytes_recover_when_osd_is_installed_without_changing_the_configuration
    cached_file_recovers_when_osd_is_installed_without_changing_the_configuration
    prose::explicit_language_block_abstains_on_extra_prose_regions
    prose::explicit_language_block_recovers_the_sideways_source_paragraph
    explicit_language_block_recovers_the_counterclockwise_source_grid
    explicit_language_block_recovers_the_upside_down_source_grid
    explicit_language_block_recovers_without_classifier_features
  )
  filters=(--exact "${expected_names[@]}")
  ;;
*)
  echo "Expected profile warm or native" >&2
  exit 1
  ;;
esac

verify_model() {
  local path="$1" expected_sha="$2" actual
  [ -s "$path" ] || {
    echo "Missing required cached model: $path" >&2
    exit 1
  }
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$path" | awk '{print $1}')"
  else
    actual="$(shasum -a 256 "$path" | awk '{print $1}')"
  fi
  [ "$actual" = "$expected_sha" ] || {
    echo "Cached model SHA256 mismatch: expected $expected_sha, observed $actual" >&2
    exit 1
  }
}

export XBERG_OSD_TEST_MODEL="${XBERG_OSD_TEST_MODEL:?Pinned cached OSD model required}"
verify_model "$XBERG_OSD_TEST_MODEL" 9cf5d576fcc47564f11265841e5ca839001e7e6f38ff7f7aacf46d15a96b00ff
if [ "$profile" = warm ]; then
  verify_model "$XBERG_ORIENTATION_TEST_MODEL" 6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947
fi
# Every model child also verifies the bundled English SHA before seeding its private offline cache. ~keep
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
export HF_HUB_OFFLINE=1
export HUGGINGFACE_HUB_OFFLINE=1
log_dir="$(mktemp -d)"
trap 'rm -rf "$log_dir"' EXIT
cargo_args=(test --locked -p xberg --no-default-features --features "$features"
  --test ocr_explicit_sideways_2496)
cargo "${cargo_args[@]}" -- "${filters[@]}" --include-ignored --list --color never 2>&1 | tee "$log_dir/list.log"
observed="$(awk '/: test$/ { count++ } END { print count + 0 }' "$log_dir/list.log")"
[ "$observed" -eq "$expected" ] || {
  echo "Explicit sideways $profile: expected $expected collected tests, observed $observed" >&2
  exit 1
}
for name in "${expected_names[@]}"; do
  grep -qxF "$name: test" "$log_dir/list.log" || {
    echo "Explicit sideways $profile: required test was not collected: $name" >&2
    exit 1
  }
done
cargo "${cargo_args[@]}" -- "${filters[@]}" --include-ignored --test-threads=1 --color never \
  2>&1 | tee "$log_dir/run.log"
grep -qF "test result: ok. $expected passed; 0 failed; 0 ignored;" "$log_dir/run.log" || {
  echo "Explicit sideways $profile: expected $expected executed tests with none ignored" >&2
  exit 1
}
echo "Explicit sideways $profile: expected $expected, passed $expected"
