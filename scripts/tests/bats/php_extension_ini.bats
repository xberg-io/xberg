#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/packages/php" "$XBERG_WORK/target/release" "$XBERG_WORK/extensions"
	cd "$XBERG_WORK/packages/php" || return
	export TEST_EXTENSION_DIR="$XBERG_WORK/extensions"
	xberg_stub php 'printf "%s\n" "$TEST_EXTENSION_DIR"'
}

@test "PHP extension ini setup should reject a missing built extension without writing configuration" {
	run bash "$REPOSITORY_ROOT/scripts/setup-php-ext-ini.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'extension not found in target/release/'
	[ ! -e php.ini ]
}

@test "PHP extension ini setup should install the selected artifact and write its exact basename" {
	printf 'native extension\n' >"$XBERG_WORK/target/release/libxberg_php.so"
	run bash "$REPOSITORY_ROOT/scripts/setup-php-ext-ini.sh"
	xberg_assert_status 0
	xberg_assert_file "$TEST_EXTENSION_DIR/libxberg_php.so" 'native extension'
	xberg_assert_file php.ini "; Temporary PHP INI for e2e tests — loads xberg PHP extension from system extension directory
[PHP]
extension_dir=$TEST_EXTENSION_DIR
extension=libxberg_php.so"
}

@test "PHP extension ini setup should propagate installation failure without claiming success or writing configuration" {
	printf 'native extension\n' >"$XBERG_WORK/target/release/libxberg_php.so"
	xberg_stub_exit cp 9
	run bash "$REPOSITORY_ROOT/scripts/setup-php-ext-ini.sh"
	xberg_assert_status 9
	[ ! -e php.ini ]
	[[ "$output" != *'Extension copied/verified'* ]]
}
