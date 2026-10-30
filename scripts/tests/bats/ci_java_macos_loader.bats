#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p target/release
	printf 'FFI payload\n' >target/release/libxberg_ffi.dylib
	printf 'ORT payload\n' >target/release/libonnxruntime.1.dylib
	export INSTALL_NAME_STATUS=0 OTOOL_STATUS=0 OTOOL_FIXTURE="$BATS_TEST_TMPDIR/otool"
	printf '%s\n' 'target/release/libxberg_ffi.dylib:' \
		'  @loader_path/libpdfium.dylib (compatibility version 0.0.0)' \
		'  @loader_path/libonnxruntime.1.dylib (compatibility version 0.0.0)' >"$OTOOL_FIXTURE"
	xberg_stub uname 'printf "Darwin\n"'
	xberg_stub install_name_tool 'printf "%s\n" "$*" >>"$XBERG_TRACE"; exit "$INSTALL_NAME_STATUS"'
	xberg_stub otool 'cat "$OTOOL_FIXTURE"; exit "$OTOOL_STATUS"'
}

@test "Java macOS loader fix should rewrite PDFium and each staged ORT dependency beside the FFI" {
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	xberg_assert_status 0
	xberg_assert_trace '-change @rpath/libpdfium.dylib @loader_path/libpdfium.dylib target/release/libxberg_ffi.dylib
-change @rpath/libonnxruntime.1.dylib @loader_path/libonnxruntime.1.dylib target/release/libxberg_ffi.dylib'
	xberg_assert_file target/release/libxberg_ffi.dylib 'FFI payload'
}

@test "Java macOS loader fix should skip other platforms and an absent FFI without touching tools" {
	xberg_stub uname 'printf "Linux\n"'
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	xberg_assert_status 0
	[ ! -e "$XBERG_TRACE" ]
	xberg_stub uname 'printf "Darwin\n"'
	rm target/release/libxberg_ffi.dylib
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	xberg_assert_status 0
	[ ! -e "$XBERG_TRACE" ]
}

@test "Java macOS loader fix should preserve a failed install name rewrite" {
	export INSTALL_NAME_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	xberg_assert_status 7
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "Java macOS loader fix should reject residual rpath dependencies after successful rewrites" {
	printf '%s\n' 'target/release/libxberg_ffi.dylib:' \
		'  @rpath/libonnxruntime.1.dylib (compatibility version 0.0.0)' >"$OTOOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'still resolves these through @rpath after rewriting'
	xberg_assert_output_contains '@rpath/libonnxruntime.1.dylib'
}

@test "Java macOS loader fix should reject a failed load-command inspection" {
	export OTOOL_STATUS=9
	: >"$OTOOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/java/fix-macos-dylib-references.sh"
	[ "$status" -ne 0 ]
}
