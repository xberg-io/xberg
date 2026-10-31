#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export RUNNER_TEMP="$XBERG_WORK/runner" SWAP_SIZE_GIB=1 AVAILABLE_KIB=16777216 SHOW_SWAP=true
	mkdir -p "$RUNNER_TEMP"
	xberg_stub uname 'printf "Linux\n"'
	xberg_stub df 'printf "Filesystem 1024-blocks Used Available Capacity Mounted\nfixture 20000000 0 %s 0%% /\n" "$AVAILABLE_KIB"'
	xberg_stub free 'printf "memory fixture\n"'
	xberg_stub sudo \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'if [ "${1:-}" = swapon ] && [ "${2:-}" = --show=NAME ] && [ "$SHOW_SWAP" = true ]; then printf "%s/xberg-benchmark-build.swap\n" "$RUNNER_TEMP"; fi'
}

@test "benchmark swap setup should allocate one selected file and verify that exact active path" {
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 0
	xberg_assert_trace "fallocate --length 1G $RUNNER_TEMP/xberg-benchmark-build.swap
chmod 600 $RUNNER_TEMP/xberg-benchmark-build.swap
mkswap $RUNNER_TEMP/xberg-benchmark-build.swap
swapon $RUNNER_TEMP/xberg-benchmark-build.swap
swapon --show=NAME --noheadings"
	xberg_assert_output_contains "activated 1 GiB benchmark build swap at $RUNNER_TEMP/xberg-benchmark-build.swap"
}

@test "benchmark swap setup should reject non-Linux hosts before privileged commands" {
	xberg_stub uname 'printf "Darwin\n"'
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'requires Linux'
	[ ! -e "$XBERG_TRACE" ]
}

@test "benchmark swap setup should reject invalid sizes before privileged commands" {
	for value in 0 -1 invalid 1.5; do
		run env SWAP_SIZE_GIB="$value" bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
		xberg_assert_status 1
		xberg_assert_output_contains 'SWAP_SIZE_GIB must be a positive integer'
	done
	[ ! -e "$XBERG_TRACE" ]
}

@test "benchmark swap setup should reject a missing temporary directory and an existing swap path" {
	run env RUNNER_TEMP="$XBERG_WORK/missing" bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'RUNNER_TEMP must name an existing directory'
	printf 'preserved contents\n' >"$RUNNER_TEMP/xberg-benchmark-build.swap"
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'refusing to overwrite existing swap path'
	xberg_assert_file "$RUNNER_TEMP/xberg-benchmark-build.swap" 'preserved contents'
	[ ! -e "$XBERG_TRACE" ]
}

@test "benchmark swap setup should retain its eight GiB disk reserve before allocating" {
	export AVAILABLE_KIB=9437183
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'insufficient disk for 1 GiB swap plus 8 GiB build reserve'
	[ ! -e "$XBERG_TRACE" ]
}

@test "benchmark swap setup should reject a false-success activation without the expected swap row" {
	export SHOW_SWAP=false
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/setup-build-swap.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'swap activation did not expose the expected path'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 5 ]
}
