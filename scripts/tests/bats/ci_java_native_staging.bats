#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p target/release
}

@test "Java collector should stage only Linux libraries for both Linux architectures" {
	printf 'linux ffi\n' >target/release/libxberg_ffi.so
	printf 'linux pdfium\n' >target/release/libpdfium.so
	printf 'foreign\n' >target/release/xberg_ffi.dll
	for rid in linux-x86_64 linux-arm64; do
		run bash "$REPOSITORY_ROOT/scripts/ci/java/collect-native-artifacts.sh" "$rid"
		xberg_assert_status 0
		xberg_assert_file "java-natives/$rid/libxberg_ffi.so" 'linux ffi'
		xberg_assert_file "java-natives/$rid/libpdfium.so" 'linux pdfium'
		[ "$(find "java-natives/$rid" -type f | wc -l | tr -d '[:space:]')" -eq 2 ]
	done
}

@test "Java collector should stage Windows FFI without requiring optional PDFium" {
	printf 'windows ffi\n' >target/release/xberg_ffi.dll
	run bash "$REPOSITORY_ROOT/scripts/ci/java/collect-native-artifacts.sh" windows-x86_64 'output with spaces'
	xberg_assert_status 0
	xberg_assert_file 'output with spaces/xberg_ffi.dll' 'windows ffi'
	[ "$(find 'output with spaces' -type f | wc -l | tr -d '[:space:]')" -eq 1 ]
}

@test "Java collector should stage macOS dylibs for both supported architectures" {
	printf 'mac ffi\n' >target/release/libxberg_ffi.dylib
	printf 'mac pdfium\n' >target/release/libpdfium.dylib
	for rid in macos-x86_64 macos-arm64; do
		run bash "$REPOSITORY_ROOT/scripts/ci/java/collect-native-artifacts.sh" "$rid"
		xberg_assert_status 0
		xberg_assert_file "java-natives/$rid/libxberg_ffi.dylib" 'mac ffi'
		xberg_assert_file "java-natives/$rid/libpdfium.dylib" 'mac pdfium'
	done
}

@test "Java collector should reject an unsupported RID and a missing required binary" {
	run bash "$REPOSITORY_ROOT/scripts/ci/java/collect-native-artifacts.sh" solaris-sparc
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported rid: solaris-sparc'
	run bash "$REPOSITORY_ROOT/scripts/ci/java/collect-native-artifacts.sh" linux-x86_64
	[ "$status" -ne 0 ]
	[ ! -e java-natives/linux-x86_64/libxberg_ffi.so ]
}

@test "Java resource staging should support direct and nested downloaded artifact layouts" {
	mkdir -p artifacts/java-natives/java-natives-linux-x86_64 \
		artifacts/java-natives/download/macos-arm64 \
		artifacts/java-natives/download/windows-x86_64
	printf 'linux\n' >artifacts/java-natives/java-natives-linux-x86_64/libxberg_ffi.so
	printf 'mac\n' >artifacts/java-natives/download/macos-arm64/libxberg_ffi.dylib
	printf 'windows\n' >artifacts/java-natives/download/windows-x86_64/xberg_ffi.dll
	printf 'ignore\n' >artifacts/java-natives/java-natives-linux-x86_64/.gitkeep
	printf 'ignore\n' >artifacts/java-natives/download/macos-arm64/.gitkeep
	printf 'not a directory\n' >artifacts/java-natives/README
	run bash "$REPOSITORY_ROOT/scripts/ci/java/stage-native-resources.sh"
	xberg_assert_status 0
	xberg_assert_file packages/java/src/main/resources/natives/linux-x86_64/libxberg_ffi.so linux
	xberg_assert_file packages/java/src/main/resources/natives/macos-arm64/libxberg_ffi.dylib mac
	xberg_assert_file packages/java/src/main/resources/natives/windows-x86_64/xberg_ffi.dll windows
	[ "$(find packages/java/src/main/resources/natives -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
}

@test "Java resource staging should fail when the downloaded artifact directory is absent" {
	run bash "$REPOSITORY_ROOT/scripts/ci/java/stage-native-resources.sh" absent
	[ "$status" -ne 0 ]
	[ "$(find packages/java/src/main/resources/natives -type f | wc -l | tr -d '[:space:]')" -eq 0 ]
}
