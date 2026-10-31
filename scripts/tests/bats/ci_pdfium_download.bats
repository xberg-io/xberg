#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	export RUNNER_TEMP="$BATS_TEST_TMPDIR/runner" TMPDIR="$BATS_TEST_TMPDIR/tmp"
	mkdir -p "$RUNNER_TEMP" "$TMPDIR" "$BATS_TEST_TMPDIR/source/lib"
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export PDFIUM_VERSION=7678 PDFIUM_FIXTURE_ARCHIVE="$BATS_TEST_TMPDIR/pdfium.tgz"
	unset PLATFORM_ID ARCH_ID PDFIUM_PLATFORM PDFIUM_ARCH CURL_STATUS
	printf 'actual archive payload\n' >"$BATS_TEST_TMPDIR/source/lib/libpdfium.so"
	tar -czf "$PDFIUM_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/source" lib
	xberg_stub uname 'if [ "$1" = -s ]; then echo Linux; else echo aarch64; fi'
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$PDFIUM_FIXTURE_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
}

@test "PDFium download should detect Linux ARM and stage the actual extracted archive" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	xberg_assert_status 0
	xberg_assert_file "$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so" 'actual archive payload'
	xberg_assert_github_output "pdfium_path=$RUNNER_TEMP/pdfium-prebuilt"
	grep -Fq 'chromium/7678/pdfium-linux-arm64.tgz' "$XBERG_TRACE"
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "PDFium download should prefer explicit platform and architecture over detected values" {
	export PLATFORM_ID=mac ARCH_ID=x64 PDFIUM_PLATFORM=linux PDFIUM_ARCH=arm64
	mkdir -p "$RUNNER_TEMP/pdfium-prebuilt"
	printf 'stale\n' >"$RUNNER_TEMP/pdfium-prebuilt/stale"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	xberg_assert_status 0
	grep -Fq 'chromium/7678/pdfium-mac-x64.tgz' "$XBERG_TRACE"
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt/stale" ]
	xberg_assert_file "$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so" 'actual archive payload'
}

@test "PDFium download should reject a missing release version before network access" {
	unset PDFIUM_VERSION
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	xberg_assert_status 2
	xberg_assert_output_contains 'PDFIUM_VERSION env var required'
	xberg_assert_github_output ''
	[ ! -e "$XBERG_TRACE" ]
}

@test "PDFium download should reject unsupported detected operating systems before network access" {
	xberg_stub uname 'printf "FreeBSD\n"'
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported platform: FreeBSD'
	[ ! -e "$XBERG_TRACE" ]
}

@test "PDFium download should preserve download failures and export no staging directory" {
	export CURL_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	xberg_assert_status 7
	xberg_assert_github_output ''
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt" ]
}

@test "PDFium download should reject a malformed archive without publishing a path" {
	printf 'not a gzip archive\n' >"$PDFIUM_FIXTURE_ARCHIVE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh"
	[ "$status" -ne 0 ]
	xberg_assert_github_output ''
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt" ]
}
