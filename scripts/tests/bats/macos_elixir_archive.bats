#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
}

@test "macOS Elixir archive contracts run the complete portable unittest suite" {
	run env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s "$REPO_ROOT/scripts/ci" -p test_macos_elixir_archive.py
	xberg_assert_status 0
	# 25 MacosElixirArchiveTests + 4 ProjectNoticeTests in scripts/ci/test_macos_elixir_archive.py.
	# A discovery change that silently stops collecting a class is what this exact count catches.
	xberg_assert_output_contains "Ran 29 tests"
	xberg_assert_output_contains "OK"
}
