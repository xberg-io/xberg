#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/scripts" "$XBERG_WORK/target/release"
	cp "$REPOSITORY_ROOT/scripts/stage_csharp_native_local.sh" "$REPOSITORY_ROOT/scripts/stage_go_native_local.sh" "$XBERG_WORK/scripts/"
	export STAGING_SYSTEM=Linux STAGING_ARCH=x86_64
	xberg_stub uname 'case "$1" in -s) printf "%s\n" "$STAGING_SYSTEM" ;; -m) printf "%s\n" "$STAGING_ARCH" ;; *) exit 97 ;; esac'
}

assert_staging() {
	local system="$1" arch="$2" filename="$3" csharp="$4" go="$5"
	export STAGING_SYSTEM="$system" STAGING_ARCH="$arch"
	local csharp_file="$XBERG_WORK/packages/csharp/Xberg/runtimes/$csharp/native/$filename"
	local go_file="$XBERG_WORK/packages/go/.lib/$go/$filename"
	local payload="$system $arch $filename"
	rm -f "$csharp_file" "$go_file"
	[ ! -e "$csharp_file" ]
	[ ! -e "$go_file" ]
	printf '%s csharp\n' "$payload" >"$XBERG_WORK/target/release/$filename"
	run bash "$XBERG_WORK/scripts/stage_csharp_native_local.sh"
	xberg_assert_status 0
	xberg_assert_file "$csharp_file" "$payload csharp"
	printf '%s go\n' "$payload" >"$XBERG_WORK/target/release/$filename"
	run bash "$XBERG_WORK/scripts/stage_go_native_local.sh"
	xberg_assert_status 0
	xberg_assert_file "$go_file" "$payload go"
}

@test "local native staging should preserve Linux x64 artifacts in both package layouts" {
	assert_staging Linux x86_64 libxberg_ffi.so linux-x64 linux-x86_64
}

@test "local native staging should normalize both Linux ARM architecture names" {
	assert_staging Linux aarch64 libxberg_ffi.so linux-arm64 linux-aarch64
	assert_staging Linux arm64 libxberg_ffi.so linux-arm64 linux-aarch64
}

@test "local native staging should preserve Intel macOS artifacts in both package layouts" {
	assert_staging Darwin x86_64 libxberg_ffi.dylib osx-x64 macos-x86_64
}

@test "local native staging should normalize both macOS ARM architecture names" {
	assert_staging Darwin arm64 libxberg_ffi.dylib osx-arm64 macos-arm64
	assert_staging Darwin aarch64 libxberg_ffi.dylib osx-arm64 macos-arm64
}

@test "local native staging should use the unprefixed Windows DLL for every supported shell" {
	assert_staging MINGW64_NT x86_64 xberg_ffi.dll win-x64 windows-x86_64
	assert_staging MSYS_NT x86_64 xberg_ffi.dll win-x64 windows-x86_64
	assert_staging CYGWIN_NT x86_64 xberg_ffi.dll win-x64 windows-x86_64
}

@test "local native staging should reject unsupported systems before creating package directories" {
	export STAGING_SYSTEM=FreeBSD
	run bash "$XBERG_WORK/scripts/stage_csharp_native_local.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported platform: FreeBSD'
	run bash "$XBERG_WORK/scripts/stage_go_native_local.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported platform: FreeBSD'
	[ ! -e "$XBERG_WORK/packages" ]
}

@test "local native staging should reject missing built libraries before creating package directories" {
	run bash "$XBERG_WORK/scripts/stage_csharp_native_local.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'target/release/libxberg_ffi.so not found'
	run bash "$XBERG_WORK/scripts/stage_go_native_local.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'target/release/libxberg_ffi.so not found'
	[ ! -e "$XBERG_WORK/packages" ]
}

@test "local native staging should propagate failed artifact copies in both package layouts" {
	printf 'native fixture\n' >"$XBERG_WORK/target/release/libxberg_ffi.so"
	xberg_stub_exit cp 9
	run bash "$XBERG_WORK/scripts/stage_csharp_native_local.sh"
	xberg_assert_status 9
	[[ "$output" != *Staged* ]]
	run bash "$XBERG_WORK/scripts/stage_go_native_local.sh"
	xberg_assert_status 9
	[[ "$output" != *Staged* ]]
}
