#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts source/lib source/include temporary
	cp "$REPOSITORY_ROOT/scripts/download_pdfium_runtime.sh" scripts/
	export PDFIUM_ARCHIVE="$XBERG_WORK/fixture.tgz" PREFIX="$XBERG_WORK/prefix"
	export TMPDIR="$XBERG_WORK/temporary" PDFIUM_VERSION=7678 PKG_CONFIG_PATH=''
	export PDFIUM_SYSTEM=Linux PDFIUM_ARCH=aarch64 REAL_CP
	REAL_CP="$(command -v cp)"
	unset CURL_STATUS HEADER_COPY_STATUS
	printf 'Linux runtime bytes\n' >source/lib/libpdfium.so
	printf 'PDFium header bytes\n' >source/include/fpdfview.h
	tar -czf "$PDFIUM_ARCHIVE" -C source lib include
	xberg_stub uname 'case "$1" in -s) printf "%s\n" "$PDFIUM_SYSTEM" ;; -m) printf "%s\n" "$PDFIUM_ARCH" ;; *) exit 97 ;; esac'
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then "$REAL_CP" "$PDFIUM_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
	xberg_stub mktemp 'mkdir -p "$TMPDIR/download"; printf "%s\n" "$TMPDIR/download"'
	xberg_stub sudo 'exec "$@"'
	xberg_stub_trace ldconfig
	xberg_stub pkg-config 'printf "7678\n"'
}

prepare_macos_archive() {
	export PDFIUM_SYSTEM=Darwin PDFIUM_ARCH=arm64
	rm source/lib/libpdfium.so
	printf 'macOS runtime bytes\n' >source/lib/libpdfium.dylib
	tar -czf "$PDFIUM_ARCHIVE" -C source lib include
}

@test "local PDFium downloader should stage actual archive bytes to both build profiles" {
	run bash "$XBERG_WORK/scripts/download_pdfium_runtime.sh"
	xberg_assert_status 0
	xberg_assert_file target/release/libpdfium.so 'Linux runtime bytes'
	xberg_assert_file target/debug/libpdfium.so 'Linux runtime bytes'
	grep -Fq 'chromium/7678/pdfium-linux-arm64.tgz' "$XBERG_TRACE"
	[ "$(find target -type f | wc -l | tr -d '[:space:]')" -eq 2 ]
	[ -z "$(find temporary -mindepth 1 -print -quit)" ]
}

@test "local PDFium downloader should preserve download failure and clean its temporary directory" {
	export CURL_STATUS=7
	run bash "$XBERG_WORK/scripts/download_pdfium_runtime.sh"
	xberg_assert_status 7
	[ ! -e target ]
	[ -z "$(find temporary -mindepth 1 -print -quit)" ]
}

@test "local PDFium downloader should reject archives without runtime libraries before creating build outputs" {
	tar -czf "$PDFIUM_ARCHIVE" -C source include
	run bash "$XBERG_WORK/scripts/download_pdfium_runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'archive did not contain lib directory'
	[ ! -e target ]
}

@test "local PDFium downloader should reject unknown architectures before downloading" {
	export PDFIUM_ARCH=riscv64
	run bash "$XBERG_WORK/scripts/download_pdfium_runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported architecture: riscv64'
	xberg_assert_trace_empty
}

@test "Linux PDFium system installer should install exact runtime and headers with matching package metadata" {
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-linux.sh" "$PREFIX" 7678
	xberg_assert_status 0
	xberg_assert_file "$PREFIX/lib/libpdfium.so" 'Linux runtime bytes'
	xberg_assert_file "$PREFIX/include/pdfium/fpdfview.h" 'PDFium header bytes'
	grep -Fxq "prefix=$PREFIX" "$PREFIX/lib/pkgconfig/pdfium.pc"
	grep -Fxq 'Version: 7678' "$PREFIX/lib/pkgconfig/pdfium.pc"
	grep -Fq 'chromium/7678/pdfium-linux-x64.tgz' "$XBERG_TRACE"
	grep -Fxq 'ldconfig ' "$XBERG_TRACE"
	[ "$(find "$PREFIX" -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
}

@test "Linux PDFium system installer should reject failed downloads before installing files" {
	export CURL_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-linux.sh" "$PREFIX"
	xberg_assert_status 1
	xberg_assert_output_contains 'Failed to download PDFium 7678'
	[ ! -e "$PREFIX" ]
}

@test "Linux PDFium system installer should reject a missing runtime instead of publishing package metadata" {
	tar -czf "$PDFIUM_ARCHIVE" -C source include
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-linux.sh" "$PREFIX"
	xberg_assert_status 1
	xberg_assert_output_contains 'libpdfium.so not found in archive'
	[ ! -e "$PREFIX/lib/pkgconfig/pdfium.pc" ]
}

@test "Linux PDFium system installer should propagate native copy errors without printing completion" {
	xberg_stub_exit cp 9
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-linux.sh" "$PREFIX"
	xberg_assert_status 9
	[[ "$output" != *'Installation complete!'* ]]
	[ ! -e "$PREFIX/lib/pkgconfig/pdfium.pc" ]
}

@test "macOS PDFium system installer should install exact library and header bytes with matching metadata" {
	prepare_macos_archive
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-macos.sh"
	xberg_assert_status 0
	xberg_assert_file "$PREFIX/lib/libpdfium.dylib" 'macOS runtime bytes'
	xberg_assert_file "$PREFIX/include/pdfium/fpdfview.h" 'PDFium header bytes'
	grep -Fxq "prefix=$PREFIX" "$PREFIX/lib/pkgconfig/pdfium.pc"
	grep -Fxq 'Version: 7678' "$PREFIX/lib/pkgconfig/pdfium.pc"
	grep -Fq 'chromium/7678/pdfium-mac-arm64.tgz' "$XBERG_TRACE"
	[ "$(find "$PREFIX" -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
	[ ! -e "$TMPDIR/download" ]
}

@test "macOS PDFium system installer should allow an initially unset pkg-config search path" {
	prepare_macos_archive
	unset PKG_CONFIG_PATH
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-macos.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Installation complete!'
}

@test "macOS PDFium system installer should reject an archive with zero shared libraries" {
	prepare_macos_archive
	rm source/lib/libpdfium.dylib
	tar -czf "$PDFIUM_ARCHIVE" -C source lib include
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-macos.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'No PDFium shared libraries found in archive'
	[ ! -e "$PREFIX/lib/pkgconfig/pdfium.pc" ]
}

@test "macOS PDFium system installer should propagate header copy failures before publishing metadata" {
	prepare_macos_archive
	xberg_stub cp \
		'if [ "$1" = -r ]; then exit 9; fi' \
		'exec "$REAL_CP" "$@"'
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-macos.sh"
	xberg_assert_status 9
	[[ "$output" != *'Header installation complete'* ]]
	[ ! -e "$PREFIX/lib/pkgconfig/pdfium.pc" ]
}

@test "macOS PDFium system installer should reject non-macOS hosts before downloading" {
	run bash "$REPOSITORY_ROOT/scripts/install-system-pdfium-macos.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'This script is for macOS only. Detected: Linux'
	xberg_assert_trace_empty
}
