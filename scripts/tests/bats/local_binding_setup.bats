#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p target/release extensions
	export TEST_EXTENSION_DIR="$XBERG_WORK/extensions" TEST_PHP_INI="$XBERG_WORK/php.ini"
	printf '[PHP]\n' >"$TEST_PHP_INI"
	xberg_stub php 'case "$*" in *extension_dir*) printf "%s\n" "$TEST_EXTENSION_DIR" ;; *php_ini_loaded_file*) printf "%s\n" "$TEST_PHP_INI" ;; *) exit 97 ;; esac'
}

@test "PHP extension installer should copy exact native bytes and register its basename once" {
	printf 'shared extension\n' >target/release/libxberg_php.so
	run bash "$REPOSITORY_ROOT/scripts/install-php-ext.sh"
	xberg_assert_status 0
	xberg_assert_file extensions/libxberg_php.so 'shared extension'
	xberg_assert_file php.ini '[PHP]
extension=libxberg_php.so'
	run bash "$REPOSITORY_ROOT/scripts/install-php-ext.sh"
	xberg_assert_status 0
	xberg_assert_file php.ini '[PHP]
extension=libxberg_php.so'
}

@test "PHP extension installer should prefer the macOS artifact when multiple platforms are present" {
	printf 'macOS extension\n' >target/release/libxberg_php.dylib
	printf 'Linux extension\n' >target/release/libxberg_php.so
	run bash "$REPOSITORY_ROOT/scripts/install-php-ext.sh"
	xberg_assert_status 0
	xberg_assert_file extensions/libxberg_php.dylib 'macOS extension'
	[ ! -e extensions/libxberg_php.so ]
}

@test "PHP extension installer should reject absent native artifacts before modifying the ini file" {
	run bash "$REPOSITORY_ROOT/scripts/install-php-ext.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'PHP extension not found in target/release/'
	xberg_assert_file php.ini '[PHP]'
}

@test "PHP extension installer should propagate copy failures before modifying the ini file" {
	printf 'shared extension\n' >target/release/libxberg_php.so
	xberg_stub_exit cp 9
	run bash "$REPOSITORY_ROOT/scripts/install-php-ext.sh"
	xberg_assert_status 9
	xberg_assert_file php.ini '[PHP]'
	[[ "$output" != *'Installed PHP extension'* ]]
}

make_swift_output() {
	mkdir -p target/release/build/xberg-swift-fixture/out/xberg-swift
	printf 'core header\n' >target/release/build/xberg-swift-fixture/out/SwiftBridgeCore.h
	printf 'bridge header\n' >target/release/build/xberg-swift-fixture/out/xberg-swift/xberg-swift.h
	printf '    var ptr: UnsafeMutableRawPointer\n    var isOwned: Bool = true\n' >target/release/build/xberg-swift-fixture/out/SwiftBridgeCore.swift
	printf 'binding source\n' >target/release/build/xberg-swift-fixture/out/xberg-swift/xberg-swift.swift
}

@test "Swift bridge setup should concatenate headers and make bridge ownership fields public" {
	make_swift_output
	run bash "$REPOSITORY_ROOT/scripts/setup-swift-bridge.sh"
	xberg_assert_status 0
	xberg_assert_file packages/swift/Sources/RustBridgeC/RustBridgeC.h 'core header
bridge header'
	xberg_assert_file packages/swift/Sources/RustBridge/SwiftBridgeCore.swift 'import RustBridgeC
    public var ptr: UnsafeMutableRawPointer
    public var isOwned: Bool = true'
	xberg_assert_file packages/swift/Sources/RustBridge/xberg-swift.swift 'import RustBridgeC
binding source'
	[ "$(find packages/swift -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
}

@test "Swift bridge setup should reject an absent build directory before writing package files" {
	run bash "$REPOSITORY_ROOT/scripts/setup-swift-bridge.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Could not find swift-bridge build output'
	[ ! -e packages ]
}

@test "Swift bridge setup should reject incomplete header output without claiming completion" {
	make_swift_output
	rm target/release/build/xberg-swift-fixture/out/xberg-swift/xberg-swift.h
	run bash "$REPOSITORY_ROOT/scripts/setup-swift-bridge.sh"
	[ "$status" -ne 0 ]
	xberg_assert_output_contains 'xberg-swift.h'
	[[ "$output" != *'setup complete'* ]]
}

@test "Swift bridge setup should preserve formatter failures without claiming completion" {
	make_swift_output
	xberg_stub_exit sed 17
	run bash "$REPOSITORY_ROOT/scripts/setup-swift-bridge.sh"
	xberg_assert_status 17
	[[ "$output" != *'setup complete'* ]]
}
