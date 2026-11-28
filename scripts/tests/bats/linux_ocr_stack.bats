setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/install-system-deps/build-ocr-linux.sh"
	export OCR_PREFIX="$XBERG_WORK/prefix" OCR_BUILD_JOBS=1 OCR_TESSDATA_DIR="$XBERG_WORK/tessdata"
	mkdir -p "$OCR_PREFIX/bin" "$OCR_TESSDATA_DIR"
	printf fixture >"$OCR_TESSDATA_DIR/eng.traineddata"
	xberg_stub pkg-config 'case "$*" in *tesseract*) echo "${MOCK_TESS_VERSION:-5.5.5}" ;; *lept*) echo 1.87.0 ;; esac'
	cat >"$OCR_PREFIX/bin/tesseract" <<'SH'
#!/usr/bin/env bash
if [ "$1" = --version ]; then printf 'tesseract %s\n leptonica-1.87.0\n' "${MOCK_TESS_VERSION:-5.5.5}"; else printf 'List of available languages (1):\neng\n'; fi
SH
	chmod +x "$OCR_PREFIX/bin/tesseract"
}

@test "linux OCR stack should_accept_only_matching_cli_and_pkg_config_versions" {
	run bash -euo pipefail -c 'source "$1"; ocr_versions_match' _ "$SCRIPT"
	xberg_assert_status 0
}

@test "linux OCR stack should_reject_a_known_old_distribution_runtime" {
	export MOCK_TESS_VERSION=5.3.4
	run bash -euo pipefail -c 'source "$1"; ocr_versions_match' _ "$SCRIPT"
	xberg_assert_status 1
}

@test "linux OCR stack should_reject_corrupt_archive_before_tar_or_cmake" {
	xberg_stub curl 'while [ "$1" != -o ]; do shift; done; printf corrupt >"$2"'
	xberg_stub tar 'touch "$XBERG_WORK/extracted"'
	xberg_stub cmake 'touch "$XBERG_WORK/compiled"'
	run bash -euo pipefail -c 'source "$1"; work_dir="$2"; ocr_build_source fake 1 https://example.invalid/source 0000000000000000000000000000000000000000000000000000000000000000' _ "$SCRIPT" "$XBERG_WORK"
	xberg_assert_status 1
	xberg_assert_output_contains 'FAILED'
	[ ! -e "$XBERG_WORK/extracted" ]
	[ ! -e "$XBERG_WORK/compiled" ]
}

@test "linux OCR stack should_reject_zero_work_install_that_leaves_old_runtime" {
	export MOCK_TESS_VERSION=5.3.4
	run bash -euo pipefail -c 'source "$1"; ocr_assert_versions' _ "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected Tesseract 5.5.5 and Leptonica 1.87.0'
}

@test "linux OCR stack should_reject_the_previous_security_patch" {
	export MOCK_TESS_VERSION=5.5.3
	run bash -euo pipefail -c 'source "$1"; ocr_versions_match' _ "$SCRIPT"
	xberg_assert_status 1
}
