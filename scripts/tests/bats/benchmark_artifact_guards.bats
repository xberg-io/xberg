#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export FIXTURE_REPO="$XBERG_WORK/repository"
	mkdir -p "$FIXTURE_REPO/scripts/benchmarks" "$FIXTURE_REPO/scripts/lib" "$FIXTURE_REPO/target/release"
	cp "$REPOSITORY_ROOT/scripts/benchmarks/ensure-benchmark-harness-exists.sh" "$FIXTURE_REPO/scripts/benchmarks/"
	cp "$REPOSITORY_ROOT/scripts/benchmarks/restore-binary-permissions.sh" "$FIXTURE_REPO/scripts/benchmarks/"
	cp "$REPOSITORY_ROOT/scripts/lib/common.sh" "$FIXTURE_REPO/scripts/lib/"
	printf '[workspace]\n' >"$FIXTURE_REPO/Cargo.toml"
	export GITHUB_REF=refs/heads/fixture
	unset BINARY_PATH
}

@test "benchmark harness guard should accept an actual repository harness directory" {
	mkdir -p "$FIXTURE_REPO/tools/benchmark-harness"
	run bash "$FIXTURE_REPO/scripts/benchmarks/ensure-benchmark-harness-exists.sh"
	xberg_assert_status 0
	xberg_assert_output_contains "Benchmark harness directory verified at: $FIXTURE_REPO/tools/benchmark-harness"
}

@test "benchmark harness guard should reject a missing harness with the selected revision" {
	run bash "$FIXTURE_REPO/scripts/benchmarks/ensure-benchmark-harness-exists.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'tools/benchmark-harness not found on branch refs/heads/fixture'
}

@test "benchmark artifact guards should reject a fixture that is not a Cargo repository" {
	rm "$FIXTURE_REPO/Cargo.toml"
	for script in ensure-benchmark-harness-exists.sh restore-binary-permissions.sh; do
		run bash "$FIXTURE_REPO/scripts/benchmarks/$script"
		xberg_assert_status 1
		xberg_assert_output_contains 'REPO_ROOT validation failed'
	done
}

@test "benchmark permission restoration should make both downloaded binaries executable" {
	printf 'harness payload\n' >"$FIXTURE_REPO/target/release/benchmark-harness"
	printf 'CLI payload\n' >"$FIXTURE_REPO/target/release/xberg"
	chmod 600 "$FIXTURE_REPO/target/release/benchmark-harness" "$FIXTURE_REPO/target/release/xberg"
	run bash "$FIXTURE_REPO/scripts/benchmarks/restore-binary-permissions.sh"
	xberg_assert_status 0
	[ -x "$FIXTURE_REPO/target/release/benchmark-harness" ]
	[ -x "$FIXTURE_REPO/target/release/xberg" ]
	xberg_assert_file "$FIXTURE_REPO/target/release/benchmark-harness" 'harness payload'
	xberg_assert_file "$FIXTURE_REPO/target/release/xberg" 'CLI payload'
	[ "$(printf '%s\n' "$output" | grep -c 'Restored executable permissions')" = 2 ]
}

@test "benchmark permission restoration should honor an explicit binary path with spaces" {
	export BINARY_PATH="$XBERG_WORK/downloaded harness"
	printf 'selected payload\n' >"$BINARY_PATH"
	chmod 600 "$BINARY_PATH"
	run bash "$FIXTURE_REPO/scripts/benchmarks/restore-binary-permissions.sh"
	xberg_assert_status 0
	[ -x "$BINARY_PATH" ]
	xberg_assert_file "$BINARY_PATH" 'selected payload'
	[ "$(printf '%s\n' "$output" | grep -c 'Restored executable permissions')" = 1 ]
}

@test "benchmark permission restoration should reject a missing harness before changing the CLI" {
	printf 'CLI payload\n' >"$FIXTURE_REPO/target/release/xberg"
	chmod 600 "$FIXTURE_REPO/target/release/xberg"
	run bash "$FIXTURE_REPO/scripts/benchmarks/restore-binary-permissions.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Binary not found at'
	[ ! -x "$FIXTURE_REPO/target/release/xberg" ]
}
