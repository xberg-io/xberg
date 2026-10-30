#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export RUSTC_WRAPPER=sccache SCCACHE_GHA_ENABLED=true
	export BUILD_CALLS="$BATS_TEST_TMPDIR/build-calls"
	unset BUILD_FAILURE SECOND_FAILURE
	xberg_stub fixture-build \
		'printf "%s|%s|%s\n" "$*" "$RUSTC_WRAPPER" "$SCCACHE_GHA_ENABLED" >>"$BUILD_CALLS"' \
		'if [ "$(wc -l <"$BUILD_CALLS" | tr -d "[:space:]")" -eq 1 ] && [ -n "${BUILD_FAILURE:-}" ]; then printf "%s\n" "$BUILD_FAILURE"; exit 7; fi' \
		'exit "${SECOND_FAILURE:-0}"'
}

@test "sccache wrapper should forward build arguments exactly once after success" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-rust/build-with-sccache-fallback.sh" fixture-build --package xberg
	xberg_assert_status 0
	xberg_assert_file "$BUILD_CALLS" '--package xberg|sccache|true'
	xberg_assert_output_contains 'Build succeeded with sccache'
}

@test "sccache wrapper should retry cache failures once with both cache settings disabled" {
	export BUILD_FAILURE='sccache: error: cache storage failed'
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-rust/build-with-sccache-fallback.sh" fixture-build --locked
	xberg_assert_status 0
	xberg_assert_file "$BUILD_CALLS" '--locked|sccache|true
--locked||false'
	xberg_assert_output_contains 'Build succeeded without sccache (fallback)'
}

@test "sccache wrapper should not retry a deterministic compiler diagnostic" {
	export BUILD_FAILURE='error[E0308]: mismatched types'
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-rust/build-with-sccache-fallback.sh" fixture-build --locked
	xberg_assert_status 1
	xberg_assert_file "$BUILD_CALLS" '--locked|sccache|true'
	xberg_assert_output_contains 'Build failed'
}

@test "sccache wrapper should retain failure when the uncached retry fails" {
	export BUILD_FAILURE='sccache failed to execute compile' SECOND_FAILURE=9
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-rust/build-with-sccache-fallback.sh" fixture-build --locked
	xberg_assert_status 1
	xberg_assert_file "$BUILD_CALLS" '--locked|sccache|true
--locked||false'
	xberg_assert_output_contains 'Build failed'
}
