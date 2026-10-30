#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_setup_workspace
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$GITHUB_WORKSPACE"
	export XBERG_PDFIUM_PREBUILT="$BATS_TEST_TMPDIR/pdfium source"
	mkdir -p "$XBERG_PDFIUM_PREBUILT/lib" "$XBERG_PDFIUM_PREBUILT/bin"
}

@test "PDFium export should preserve a path containing spaces in both workflow files" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/export-path.sh" "$XBERG_PDFIUM_PREBUILT"
	xberg_assert_status 0
	xberg_assert_github_env "XBERG_PDFIUM_PREBUILT=$XBERG_PDFIUM_PREBUILT"
	xberg_assert_github_output "pdfium-path=$XBERG_PDFIUM_PREBUILT"
}

@test "PDFium export should reject an absent path without writing workflow output" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/download-pdfium/export-path.sh"
	[ "$status" -ne 0 ]
	xberg_assert_output_contains 'pdfium path required'
	xberg_assert_github_env ''
	xberg_assert_github_output ''
}

@test "PDFium staging should copy Linux runtime to every nonempty additional destination" {
	printf 'linux runtime\n' >"$XBERG_PDFIUM_PREBUILT/lib/libpdfium.so"
	run env RUNNER_OS=Linux bash "$REPOSITORY_ROOT/scripts/ci/actions/stage-pdfium-runtime/stage.sh" \
		'primary out' $'secondary one\n\nsecondary two'
	xberg_assert_status 0
	for dest in 'primary out' 'secondary one' 'secondary two'; do
		xberg_assert_file "$dest/libpdfium.so" 'linux runtime'
	done
	[ "$(cat "$GITHUB_PATH")" = '' ]
}

@test "PDFium staging should select the macOS dylib without exporting a Windows search path" {
	printf 'mac runtime\n' >"$XBERG_PDFIUM_PREBUILT/lib/libpdfium.dylib"
	run env RUNNER_OS=macOS bash "$REPOSITORY_ROOT/scripts/ci/actions/stage-pdfium-runtime/stage.sh" dylibs
	xberg_assert_status 0
	xberg_assert_file dylibs/libpdfium.dylib 'mac runtime'
	[ "$(cat "$GITHUB_PATH")" = '' ]
}

@test "PDFium staging should normalize the Windows import library and export its runtime path" {
	printf 'windows runtime\n' >"$XBERG_PDFIUM_PREBUILT/bin/pdfium.dll"
	printf 'import library\n' >"$XBERG_PDFIUM_PREBUILT/lib/pdfium.dll.lib"
	run env RUNNER_OS=Windows bash "$REPOSITORY_ROOT/scripts/ci/actions/stage-pdfium-runtime/stage.sh" 'windows out'
	xberg_assert_status 0
	xberg_assert_file 'windows out/pdfium.dll' 'windows runtime'
	xberg_assert_file "$XBERG_PDFIUM_PREBUILT/lib/pdfium.lib" 'import library'
	xberg_assert_file "$GITHUB_PATH" "$GITHUB_WORKSPACE/windows out"
}

@test "PDFium staging should reject missing source metadata and unsupported platforms before copying" {
	run env -u XBERG_PDFIUM_PREBUILT RUNNER_OS=Linux \
		bash "$REPOSITORY_ROOT/scripts/ci/actions/stage-pdfium-runtime/stage.sh" out
	xberg_assert_status 1
	xberg_assert_output_contains 'XBERG_PDFIUM_PREBUILT is not set'
	[ ! -e out ]
	run env RUNNER_OS=FreeBSD bash "$REPOSITORY_ROOT/scripts/ci/actions/stage-pdfium-runtime/stage.sh" out
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported RUNNER_OS: FreeBSD'
	[ ! -e out ]
}
