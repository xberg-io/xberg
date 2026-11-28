setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/install-system-deps/verify-ocr-macos.sh"
	xberg_stub brew 'printf "/selected/%s\n" "$2"'
	xberg_stub pkg-config 'case "$2" in tesseract) echo "${MOCK_TESS_PC:-5.5.5}" ;; lept) echo "${MOCK_LEPT_PC:-1.87.0}" ;; esac'
	xberg_stub tesseract 'printf "tesseract %s\n leptonica-%s\n" "${MOCK_TESS_CLI:-5.5.5}" "${MOCK_LEPT_CLI:-1.87.0}"; exit "${MOCK_CLI_EXIT:-0}"'
}

@test "macOS OCR verification should_accept_only_current_cli_and_metadata" {
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains 'leptonica-1.87.0'
}

@test "macOS OCR verification should_reject_stale_cli_despite_current_metadata" {
	export MOCK_TESS_CLI=5.5.0
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected actual Tesseract CLI 5.5.5'
}

@test "macOS OCR verification should_reject_stale_linked_leptonica" {
	export MOCK_LEPT_CLI=1.86.0
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected actual Leptonica runtime 1.87.0'
}

@test "macOS OCR verification should_reject_stale_metadata_and_native_failure" {
	export MOCK_LEPT_PC=1.82.0
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected Leptonica pkg-config 1.87.0'
	unset MOCK_LEPT_PC
	export MOCK_CLI_EXIT=7
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 7
}

@test "macOS OCR verification should_reject_the_previous_security_patch" {
	export MOCK_TESS_CLI=5.5.3
	run bash -euo pipefail -c 'source "$1"; verify_ocr_macos' _ "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected actual Tesseract CLI 5.5.5'
}
