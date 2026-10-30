#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p target/release crates/xberg-ffi/include \
		artifact-staging/xberg-ffi/lib artifact-staging/xberg-ffi/include artifact-staging/xberg-ffi/share/pkgconfig
	printf 'header\n' >crates/xberg-ffi/include/xberg.h
	printf 'version = "1.4.0"\n' >crates/xberg-ffi/Cargo.toml
}

write_ffi_archive() {
	printf 'static\n' >artifact-staging/xberg-ffi/lib/libxberg_ffi.a
	printf 'header\n' >artifact-staging/xberg-ffi/include/xberg.h
	printf 'Version: 1.4.0\n' >artifact-staging/xberg-ffi/share/pkgconfig/xberg-ffi.pc
	tar -czf ffi.tar.gz -C artifact-staging xberg-ffi
}

@test "Go staging should require a static library before publishing any assets" {
	run bash "$REPOSITORY_ROOT/scripts/ci/go/stage-ffi-artifacts.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Static library not found: target/release/libxberg_ffi.a'
	[ "$(find artifact-staging -type f | wc -l | tr -d '[:space:]')" -eq 0 ]
}

@test "Go staging should preserve static dynamic PDFium and header bytes with matching pkg-config metadata" {
	printf 'static\n' >target/release/libxberg_ffi.a
	printf 'ffi dynamic\n' >target/release/libxberg_ffi.so.1
	printf 'pdfium\n' >target/release/libpdfium.so
	run bash "$REPOSITORY_ROOT/scripts/ci/go/stage-ffi-artifacts.sh"
	xberg_assert_status 0
	xberg_assert_file artifact-staging/xberg-ffi/lib/libxberg_ffi.a static
	xberg_assert_file artifact-staging/xberg-ffi/lib/libxberg_ffi.so.1 'ffi dynamic'
	xberg_assert_file artifact-staging/xberg-ffi/lib/libpdfium.so pdfium
	xberg_assert_file artifact-staging/xberg-ffi/include/xberg.h header
	grep -Fxq 'Version: 1.4.0' artifact-staging/xberg-ffi/share/pkgconfig/xberg-ffi.pc
	grep -Fxq 'Libs: -L${libdir} -lxberg_ffi' artifact-staging/xberg-ffi/share/pkgconfig/xberg-ffi.pc
	[ "$(find artifact-staging -type f | wc -l | tr -d '[:space:]')" -eq 5 ]
}

@test "Go verification should accept a genuine static-only archive and remove its scratch extraction" {
	write_ffi_archive
	run bash "$REPOSITORY_ROOT/scripts/ci/go/verify-ffi-artifact.sh" ffi.tar.gz
	xberg_assert_status 0
	xberg_assert_output_contains 'Found static library: libxberg_ffi.a'
	xberg_assert_output_contains 'No dynamic libraries found - static linking only'
	xberg_assert_output_contains 'Artifact verification passed'
	[ ! -e verify-temp ]
}

@test "Go verification should reject an absent archive and missing required header" {
	run bash "$REPOSITORY_ROOT/scripts/ci/go/verify-ffi-artifact.sh" absent.tar.gz
	xberg_assert_status 1
	xberg_assert_output_contains 'Artifact not found: absent.tar.gz'
	write_ffi_archive
	rm artifact-staging/xberg-ffi/include/xberg.h
	tar -czf incomplete.tar.gz -C artifact-staging xberg-ffi
	run bash "$REPOSITORY_ROOT/scripts/ci/go/verify-ffi-artifact.sh" incomplete.tar.gz
	xberg_assert_status 1
	xberg_assert_output_contains 'Missing: xberg-ffi/include/xberg.h'
	[ ! -e verify-temp ]
}

@test "Go verification should reject an archive with a header and metadata but no static library" {
	write_ffi_archive
	rm artifact-staging/xberg-ffi/lib/libxberg_ffi.a
	tar -czf incomplete.tar.gz -C artifact-staging xberg-ffi
	run bash "$REPOSITORY_ROOT/scripts/ci/go/verify-ffi-artifact.sh" incomplete.tar.gz
	xberg_assert_status 1
	xberg_assert_output_contains 'Missing static library: libxberg_ffi.a'
	[ ! -e verify-temp ]
}
