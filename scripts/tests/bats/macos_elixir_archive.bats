#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
}

@test "macOS Elixir archive contracts run the complete portable unittest suite" {
	run env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s "$REPO_ROOT/scripts/ci" -p test_macos_elixir_archive.py
	xberg_assert_status 0
	xberg_assert_output_contains "Ran 27 tests"
	xberg_assert_output_contains "OK"
}
