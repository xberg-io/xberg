#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	export SYMBOLS_FILE="$BATS_TEST_TMPDIR/symbols"
	printf '00000000 DF *UND* 0 (GLIBC_2.28) memcpy\n' >"$SYMBOLS_FILE"
	unset OBJDUMP_FAILURE
	xberg_stub objdump \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'cat "$SYMBOLS_FILE"' \
		'exit "${OBJDUMP_FAILURE:-0}"'
	mkdir -p native crates/xberg-node/artifacts
	printf 'ffi\n' >native/libxberg_ffi.so
	printf 'ort\n' >native/libonnxruntime.so.1
	printf 'node\n' >crates/xberg-node/artifacts/xberg-node.linux-x64-gnu.node
}

@test "artifact glibc guard should inspect every native library in a genuine archive" {
	tar -czf native.tar.gz native
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native.tar.gz
	xberg_assert_status 0
	xberg_assert_output_contains 'artifact passes the glibc floor gate'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
	xberg_assert_output_contains 'OK libxberg_ffi.so'
	xberg_assert_output_contains 'OK libonnxruntime.so.1'
}

@test "artifact glibc guard should reject a too-new symbol and honor an explicit floor" {
	printf '00000000 DF *UND* 0 (GLIBC_2.34) close_range\n' >"$SYMBOLS_FILE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native
	xberg_assert_status 1
	xberg_assert_output_contains 'requires glibc 2.34 > floor 2.28'
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native 2.34
	xberg_assert_status 0
	xberg_assert_output_contains 'artifact passes the glibc floor gate'
}

@test "artifact glibc guard should reject newer unversioned libc helpers and absent ORT" {
	printf '00000000 DF *UND* 0 __libc_single_threaded\n' >"$SYMBOLS_FILE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native
	xberg_assert_status 1
	xberg_assert_output_contains 'references too-new symbols: __libc_single_threaded'
	printf '00000000 DF *UND* 0 (GLIBC_2.17) memcpy\n' >"$SYMBOLS_FILE"
	rm native/libonnxruntime.so.1
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native
	xberg_assert_status 1
	xberg_assert_output_contains 'has no libonnxruntime.so bundled in the artifact'
}

@test "artifact glibc guard should reject an empty tree and an unreadable native library" {
	mkdir empty
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" empty
	xberg_assert_status 1
	xberg_assert_output_contains 'no native library found under'
	xberg_assert_trace_empty
	export OBJDUMP_FAILURE=8
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-glibc-floor.sh" native
	xberg_assert_status 1
	xberg_assert_output_contains 'is not a readable native library'
}

@test "Node glibc guard should inspect the selected architecture and skip other targets" {
	run env TARGET=x86_64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/verify-glibc-floor.sh"
	xberg_assert_status 0
	xberg_assert_trace '-T crates/xberg-node/artifacts/xberg-node.linux-x64-gnu.node'
	xberg_assert_output_contains 'Symbol audit PASSED'
	rm "$XBERG_TRACE"
	run env TARGET=aarch64-apple-darwin bash "$REPOSITORY_ROOT/scripts/publish/node/verify-glibc-floor.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'not a linux-gnu prebuild'
	xberg_assert_trace_empty
}

@test "Node glibc guard should reject empty symbol output instead of a zero-work pass" {
	: >"$SYMBOLS_FILE"
	run env TARGET=x86_64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/verify-glibc-floor.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'no GLIBC_* version symbols found'
}

@test "Node glibc guard should reject newer GLIBC GLIBCXX and C23 references independently" {
	for symbol in GLIBC_2.34 GLIBCXX_3.4.26 __isoc23_strtol; do
		printf '00000000 DF *UND* 0 (GLIBC_2.17) memcpy\n00000000 DF *UND* 0 %s\n' "$symbol" >"$SYMBOLS_FILE"
		run env TARGET=x86_64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/verify-glibc-floor.sh"
		xberg_assert_status 1
		xberg_assert_output_contains "$symbol"
		xberg_assert_output_contains 'refusing to package and publish'
	done
}

@test "Node glibc guard should reject an absent selected prebuild before symbol inspection" {
	run env TARGET=aarch64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/verify-glibc-floor.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'xberg-node.linux-arm64-gnu.node not found'
	xberg_assert_trace_empty
}
