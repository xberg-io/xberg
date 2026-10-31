#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	unset NODE_STATUS PNPM_STATUS LIST_STATUS
	xberg_stub node 'printf "v24.10.0\n"' 'exit "${NODE_STATUS:-0}"'
	xberg_stub pnpm \
		'if [ "${1:-}" = --version ]; then printf "12.15.0\n"; exit "${PNPM_STATUS:-0}"; fi' \
		'printf "workspace fixture\n"; exit "${LIST_STATUS:-0}"'
	xberg_stub tsx 'exit 0'
}

@test "benchmark Node setup should report both executable versions and the supplied label" {
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/verify-node-setup.sh" 'candidate setup'
	xberg_assert_status 0
	xberg_assert_output_contains '=== candidate setup ==='
	xberg_assert_output_contains 'Node version: v24.10.0'
	xberg_assert_output_contains 'pnpm version: 12.15.0'
	xberg_assert_output_contains 'workspace fixture'
}

@test "benchmark Node setup should tolerate an optional workspace listing failure" {
	export LIST_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/verify-node-setup.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Node version: v24.10.0'
	xberg_assert_output_contains 'pnpm version: 12.15.0'
}

@test "benchmark Node setup should reject an unsuccessful Node version probe" {
	export NODE_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/verify-node-setup.sh"
	[ "$status" -ne 0 ]
	[[ "$output" != *'pnpm version:'* ]]
}

@test "benchmark Node setup should reject an unsuccessful pnpm version probe" {
	export PNPM_STATUS=9
	run bash "$REPOSITORY_ROOT/scripts/ci/benchmarks/verify-node-setup.sh"
	[ "$status" -ne 0 ]
	[[ "$output" != *'pnpm workspace structure:'* ]]
}
