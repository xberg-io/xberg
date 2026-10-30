#!/usr/bin/env bats

setup_file() {
	bats_load_library xberg-bats
	xberg_shadow_system_path_without sha256sum
	[ -x "$XBERG_SYS_BIN/shasum" ]
}

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p source/nested
	printf 'alpha\n' >source/a.txt
	printf 'beta\n' >source/nested/b.txt
}

@test "cache hash should be order-independent while examining both requested files" {
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --files source/a.txt source/nested/b.txt
	xberg_assert_status 0
	first_hash="${lines[0]}"
	[[ "$first_hash" =~ ^[[:xdigit:]]{12}$ ]]
	[[ "$output" =~ Hashed[[:space:]]+2[[:space:]]files ]]
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --files source/nested/b.txt source/a.txt
	xberg_assert_status 0
	[ "${lines[0]}" = "$first_hash" ]
	[[ "$output" =~ Hashed[[:space:]]+2[[:space:]]files ]]
}

@test "cache hash should change after a selected file changes" {
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --files source/a.txt
	xberg_assert_status 0
	first_hash="${lines[0]}"
	printf 'changed\n' >source/a.txt
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --files source/a.txt
	xberg_assert_status 0
	[ "${lines[0]}" != "$first_hash" ]
	[[ "$output" =~ Hashed[[:space:]]+1[[:space:]]files ]]
}

@test "cache directory hashing should exclude generated and hidden files" {
	mkdir -p source/target source/node_modules source/.venv source/dist source/build
	for dir in target node_modules .venv dist build; do
		printf 'excluded\n' >"source/$dir/generated"
	done
	printf 'hidden\n' >source/.secret
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --dirs source
	xberg_assert_status 0
	first_hash="${lines[0]}"
	[[ "$output" =~ Hashed[[:space:]]+2[[:space:]]files ]]
	printf 'modified excluded bytes\n' >source/target/generated
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --dirs source
	xberg_assert_status 0
	[ "${lines[0]}" = "$first_hash" ]
}

@test "cache glob hashing should include matching descendants and exclude other suffixes" {
	printf 'not selected\n' >source/nested/c.rs
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" 'source/**/*.txt'
	xberg_assert_status 0
	[[ "$output" =~ Hashed[[:space:]]+2[[:space:]]files ]]
}

@test "cache hashing should reject missing inputs and unmatched patterns instead of an empty digest" {
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'No input provided'
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" --files missing
	xberg_assert_status 1
	xberg_assert_output_contains 'No files found matching the provided patterns'
	run bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" 'source/**/*.missing'
	xberg_assert_status 1
	xberg_assert_output_contains 'No files found matching the provided patterns'
}

@test "cache hashing should support the shasum fallback for files directories and recursive globs" {
	local mode_count=0
	for mode in files dirs glob; do
		case "$mode" in
		files) set -- --files source/a.txt source/nested/b.txt ;;
		dirs) set -- --dirs source ;;
		glob) set -- 'source/**/*.txt' ;;
		esac
		run env PATH="$(xberg_isolated_path)" bash "$REPOSITORY_ROOT/scripts/ci/cache/compute-hash.sh" "$@"
		xberg_assert_status 0
		[[ "${lines[0]}" =~ ^[[:xdigit:]]{12}$ ]]
		[[ "$output" =~ Hashed[[:space:]]+2[[:space:]]files ]]
		mode_count=$((mode_count + 1))
	done
	[ "$mode_count" -eq 3 ]
}
