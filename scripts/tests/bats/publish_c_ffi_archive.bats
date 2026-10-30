#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts/ci scripts/publish/c-ffi crates/xberg-ffi/include crates/xberg-ffi/cmake target/release
	cp "$REPOSITORY_ROOT/scripts/publish/c-ffi/package-c-ffi.sh" scripts/publish/c-ffi/
	cp "$REPOSITORY_ROOT/scripts/ci/package-notices.py" scripts/ci/
	printf "third-party notice bytes\n" >THIRD_PARTY_LICENSES.md
	printf 'public C header\n' >crates/xberg-ffi/include/xberg.h
	printf 'CMake imported target\n' >crates/xberg-ffi/cmake/xberg-ffi-config.cmake
	printf 'MIT license bytes\n' >LICENSE
	unset BUILD_FEATURES CARGO_BUILD_TARGET CARGO_STATUS
	xberg_stub cargo 'printf "%s\n" "$*" >>"$XBERG_TRACE"' 'exit "${CARGO_STATUS:-0}"'
}

unpack_archive() {
	local platform="$1"
	mkdir extracted
	tar -xzf "$XBERG_WORK/output/c-ffi-$platform.tar.gz" -C extracted
	export PACKAGE_ROOT="$XBERG_WORK/extracted/xberg-ffi-1.4.2-$platform"
	xberg_assert_file "$PACKAGE_ROOT/include/xberg.h" 'public C header'
	xberg_assert_file "$PACKAGE_ROOT/LICENSE" 'MIT license bytes'
	cmp THIRD_PARTY_LICENSES.md "$PACKAGE_ROOT/THIRD_PARTY_LICENSES.md"
	xberg_assert_file "$PACKAGE_ROOT/cmake/xberg-ffi-config.cmake" 'CMake imported target'
	grep -Fxq 'Version: 1.4.2' "$PACKAGE_ROOT/lib/pkgconfig/xberg.pc"
	grep -Fxq 'set(PACKAGE_VERSION_MAJOR 1)' "$PACKAGE_ROOT/cmake/xberg-ffi-config-version.cmake"
	grep -Fxq 'set(PACKAGE_VERSION_MINOR 4)' "$PACKAGE_ROOT/cmake/xberg-ffi-config-version.cmake"
	grep -Fxq 'set(PACKAGE_VERSION_PATCH 2)' "$PACKAGE_ROOT/cmake/xberg-ffi-config-version.cmake"
}

@test "C FFI packager should create a complete Linux archive with exact native bytes and metadata" {
	printf 'Linux shared library\n' >target/release/libxberg_ffi.so
	printf 'Linux static library\n' >target/release/libxberg_ffi.a
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" linux-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 0
	xberg_assert_trace 'build -p xberg-ffi --release'
	unpack_archive linux-x86_64
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.so" 'Linux shared library'
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.a" 'Linux static library'
	grep -Fxq 'Libs.private: -lpthread -ldl -lm' "$PACKAGE_ROOT/lib/pkgconfig/xberg.pc"
	[ "$(find "$PACKAGE_ROOT" -type f | wc -l | tr -d '[:space:]')" -eq 8 ]
}

@test "C FFI packager should prefer target-specific macOS libraries and preserve explicit features" {
	export CARGO_BUILD_TARGET=aarch64-apple-darwin BUILD_FEATURES=pdf,ocr
	mkdir -p "target/$CARGO_BUILD_TARGET/release"
	printf 'selected macOS shared\n' >"target/$CARGO_BUILD_TARGET/release/libxberg_ffi.dylib"
	printf 'selected macOS static\n' >"target/$CARGO_BUILD_TARGET/release/libxberg_ffi.a"
	printf 'wrong fallback shared\n' >target/release/libxberg_ffi.dylib
	printf 'wrong fallback static\n' >target/release/libxberg_ffi.a
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" macos-arm64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 0
	xberg_assert_trace 'build -p xberg-ffi --release --features pdf,ocr'
	unpack_archive macos-arm64
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.dylib" 'selected macOS shared'
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.a" 'selected macOS static'
	grep -Fxq 'Libs.private: -framework CoreFoundation -framework Security -lpthread' "$PACKAGE_ROOT/lib/pkgconfig/xberg.pc"
	[ "$(find "$PACKAGE_ROOT" -type f | wc -l | tr -d '[:space:]')" -eq 8 ]
}

@test "C FFI packager should fall back to GNU Windows artifact aliases and normalize the import library name" {
	export CARGO_BUILD_TARGET=x86_64-pc-windows-gnu
	printf 'GNU Windows DLL\n' >target/release/libxberg_ffi.dll
	printf 'GNU static library\n' >target/release/libxberg_ffi.a
	printf 'GNU import library\n' >target/release/libxberg_ffi.dll.a
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" windows-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 0
	unpack_archive windows-x86_64
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.dll" 'GNU Windows DLL'
	xberg_assert_file "$PACKAGE_ROOT/lib/libxberg_ffi.a" 'GNU static library'
	xberg_assert_file "$PACKAGE_ROOT/lib/xberg_ffi.dll.lib" 'GNU import library'
	grep -Fxq 'Libs.private: -lws2_32 -luserenv -lbcrypt' "$PACKAGE_ROOT/lib/pkgconfig/xberg.pc"
	[ "$(find "$PACKAGE_ROOT" -type f | wc -l | tr -d '[:space:]')" -eq 9 ]
}

@test "C FFI packager should reject unsupported platforms before building" {
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" freebsd-x64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unknown platform label'
	xberg_assert_trace_empty
	[ ! -e output ]
}

@test "C FFI packager should preserve build failures instead of packaging preexisting native files" {
	export CARGO_STATUS=9
	printf 'stale shared\n' >target/release/libxberg_ffi.so
	printf 'stale static\n' >target/release/libxberg_ffi.a
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" linux-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 9
	[ ! -e output ]
}

@test "C FFI packager should reject missing static libraries before creating an archive" {
	printf 'shared only\n' >target/release/libxberg_ffi.so
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" linux-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Cannot find libxberg_ffi.a'
	[ ! -e output ]
}

@test "C FFI packager should reject missing headers despite complete native libraries" {
	printf 'shared\n' >target/release/libxberg_ffi.so
	printf 'static\n' >target/release/libxberg_ffi.a
	rm crates/xberg-ffi/include/xberg.h
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" linux-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Header not found'
	[ ! -e output ]
}

@test "C FFI packager should propagate archiver failures without claiming a published archive" {
	printf 'shared\n' >target/release/libxberg_ffi.so
	printf 'static\n' >target/release/libxberg_ffi.a
	xberg_stub_exit tar 17
	run bash "$XBERG_WORK/scripts/publish/c-ffi/package-c-ffi.sh" linux-x86_64 1.4.2 "$XBERG_WORK/output"
	xberg_assert_status 17
	[[ "$output" != *'Packaged:'* ]]
	[ ! -e output/c-ffi-linux-x86_64.tar.gz ]
}
