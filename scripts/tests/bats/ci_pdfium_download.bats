#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	export RUNNER_TEMP="$BATS_TEST_TMPDIR/runner" TMPDIR="$BATS_TEST_TMPDIR/tmp"
	mkdir -p "$RUNNER_TEMP" "$TMPDIR" "$BATS_TEST_TMPDIR/source/lib"
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	DOWNLOAD_SCRIPT="$XBERG_WORK/scripts/ci/actions/download-pdfium/download.sh"
	FIXTURE_MANIFEST="$XBERG_WORK/scripts/pdfium-runtime-checksums.sha256"
	mkdir -p "$(dirname "$DOWNLOAD_SCRIPT")" "$XBERG_WORK/scripts/ci/actions/setup-onnx-runtime"
	cp "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/download.sh" "$DOWNLOAD_SCRIPT"
	cp "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/verify-archive.sh" "$XBERG_WORK/scripts/ci/actions/setup-onnx-runtime/"
	export PDFIUM_VERSION=7678 PDFIUM_FIXTURE_ARCHIVE="$BATS_TEST_TMPDIR/pdfium.tgz"
	unset PLATFORM_ID ARCH_ID PDFIUM_PLATFORM PDFIUM_ARCH PDFIUM_CHECKSUMS_FILE CURL_STATUS
	printf 'actual archive payload\n' >"$BATS_TEST_TMPDIR/source/lib/libpdfium.so"
	tar -czf "$PDFIUM_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/source" lib
	pin_fixture_archive 7678-pdfium-linux-arm64.tgz 7678-pdfium-mac-x64.tgz 7678-pdfium-win-arm64.tgz >"$FIXTURE_MANIFEST"
	xberg_stub uname 'if [ "$1" = -s ]; then echo Linux; else echo aarch64; fi'
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$PDFIUM_FIXTURE_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
}

pin_fixture_archive() {
	local checksum asset
	if command -v sha256sum >/dev/null 2>&1; then
		checksum="$(sha256sum "$PDFIUM_FIXTURE_ARCHIVE")"
	else
		checksum="$(shasum -a 256 "$PDFIUM_FIXTURE_ARCHIVE")"
	fi
	for asset in "$@"; do
		printf '%s  %s\n' "${checksum%% *}" "$asset"
	done
}

@test "PDFium download should detect Linux ARM and stage the actual extracted archive" {
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 0
	xberg_assert_file "$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so" 'actual archive payload'
	xberg_assert_github_output "pdfium_path=$RUNNER_TEMP/pdfium-prebuilt"
	grep -Fq 'chromium/7678/pdfium-linux-arm64.tgz' "$XBERG_TRACE"
	xberg_assert_output_contains '7678-pdfium-linux-arm64.tgz: SHA-256 verified'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "PDFium download should prefer explicit platform and architecture over detected values" {
	export PLATFORM_ID=mac ARCH_ID=x64 PDFIUM_PLATFORM=linux PDFIUM_ARCH=arm64
	mkdir -p "$RUNNER_TEMP/pdfium-prebuilt"
	printf 'stale\n' >"$RUNNER_TEMP/pdfium-prebuilt/stale"
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 0
	grep -Fq 'chromium/7678/pdfium-mac-x64.tgz' "$XBERG_TRACE"
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt/stale" ]
	xberg_assert_file "$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so" 'actual archive payload'
}

@test "PDFium download should reject a missing release version before network access" {
	unset PDFIUM_VERSION
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 2
	xberg_assert_output_contains 'PDFIUM_VERSION env var required'
	xberg_assert_github_output ''
	[ ! -e "$XBERG_TRACE" ]
}

@test "PDFium download should reject unsupported detected operating systems before network access" {
	xberg_stub uname 'printf "FreeBSD\n"'
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported platform: FreeBSD'
	[ ! -e "$XBERG_TRACE" ]
}

@test "PDFium download should preserve download failures and export no staging directory" {
	export CURL_STATUS=7
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 7
	xberg_assert_github_output ''
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt" ]
}

@test "PDFium download should reject a malformed archive without publishing a path" {
	printf 'not a gzip archive\n' >"$PDFIUM_FIXTURE_ARCHIVE"
	pin_fixture_archive 7678-pdfium-linux-arm64.tgz >"$FIXTURE_MANIFEST"
	run bash "$DOWNLOAD_SCRIPT"
	[ "$status" -ne 0 ]
	xberg_assert_output_contains '7678-pdfium-linux-arm64.tgz: SHA-256 verified'
	xberg_assert_github_output ''
	[ ! -e "$RUNNER_TEMP/pdfium-prebuilt" ]
}

@test "PDFium download should honor Windows detector results over uname" {
	export PDFIUM_PLATFORM=win PDFIUM_ARCH=arm64
	xberg_stub_exit uname 99
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains '7678-pdfium-win-arm64.tgz: SHA-256 verified'
	grep -Fq 'chromium/7678/pdfium-win-arm64.tgz' "$XBERG_TRACE"
	xberg_assert_github_output "pdfium_path=$RUNNER_TEMP/pdfium-prebuilt"
}

@test "PDFium download should require a trusted manifest for a custom revision" {
	export PDFIUM_VERSION=9999 PLATFORM_ID=linux-musl ARCH_ID=x64
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for 9999-pdfium-linux-musl-x64.tgz'
	xberg_assert_trace_empty
	xberg_assert_github_output ''

	export PDFIUM_CHECKSUMS_FILE="$BATS_TEST_TMPDIR/custom.sha256"
	pin_fixture_archive 9999-pdfium-linux-musl-x64.tgz | awk '{printf "%s\r\n", $0}' >"$PDFIUM_CHECKSUMS_FILE"
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains '9999-pdfium-linux-musl-x64.tgz: SHA-256 verified'
	grep -Fq 'chromium/9999/pdfium-linux-musl-x64.tgz' "$XBERG_TRACE"
}

@test "PDFium download should reject unsafe version and platform path segments before curl" {
	run env PDFIUM_VERSION='../7678' bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Invalid PDFium Chromium revision'
	run env PLATFORM_ID='../linux' bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Invalid PDFium platform or architecture'
	run env ARCH_ID='x64/extra' bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'Invalid PDFium platform or architecture'
	xberg_assert_trace_empty
	xberg_assert_github_output ''
}

@test "PDFium download should reject tampered bytes before tar and preserve live staging" {
	mkdir -p "$RUNNER_TEMP/pdfium-prebuilt/lib"
	printf 'previous runtime\n' >"$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so"
	printf 'modified runtime\n' >"$BATS_TEST_TMPDIR/source/lib/libpdfium.so"
	tar -czf "$PDFIUM_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/source" lib
	xberg_stub_exit tar 93
	run bash "$DOWNLOAD_SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'SHA-256 mismatch for 7678-pdfium-linux-arm64.tgz'
	xberg_assert_file "$RUNNER_TEMP/pdfium-prebuilt/lib/libpdfium.so" 'previous runtime'
	xberg_assert_github_output ''
	[ -z "$(find "$TMPDIR" -mindepth 1 -print -quit)" ]
}
