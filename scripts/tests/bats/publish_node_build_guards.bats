#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p crates/xberg-node/artifacts
	export TARGET=aarch64-apple-darwin USE_CROSS=false USE_NAPI_CROSS=false USE_ZIGBUILD=false
	export PNPM_STATUS=0
	unset EMIT_TYPES
	xberg_stub pnpm \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'if [ "${EMIT_TYPES:-false}" = true ]; then printf "type declarations\n" >crates/xberg-node/index.d.ts; printf "JavaScript loader\n" >crates/xberg-node/index.js; fi' \
		'exit "$PNPM_STATUS"'
}

@test "Node native builder should forward the selected target and require a generated native file" {
	printf 'native payload\n' >crates/xberg-node/artifacts/xberg.node
	run bash "$REPOSITORY_ROOT/scripts/publish/node/build-native-module.sh"
	xberg_assert_status 0
	xberg_assert_trace '--filter {./crates/xberg-node} exec napi build --platform --release --target aarch64-apple-darwin --output-dir ./artifacts'
	printf '%s\n' "$output" | grep -Eq 'Found[[:space:]]+1[[:space:]]+\.node file\(s\)$'
}

@test "Node native builder should preserve all explicit cross-compilation switches" {
	export USE_CROSS=true USE_NAPI_CROSS=true USE_ZIGBUILD=true
	printf 'native payload\n' >crates/xberg-node/artifacts/xberg.node
	run bash "$REPOSITORY_ROOT/scripts/publish/node/build-native-module.sh"
	xberg_assert_status 0
	xberg_assert_trace '--filter {./crates/xberg-node} exec napi build --platform --release --target aarch64-apple-darwin --output-dir ./artifacts --use-napi-cross --use-cross --cross-compile'
}

@test "Node native builder should reject a success-looking build that emitted no native artifacts" {
	run bash "$REPOSITORY_ROOT/scripts/publish/node/build-native-module.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Native module build succeeded but no .node file was generated'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "Node native builder should preserve a failed napi build even when an old binary exists" {
	export PNPM_STATUS=7
	printf 'stale payload\n' >crates/xberg-node/artifacts/xberg.node
	run bash "$REPOSITORY_ROOT/scripts/publish/node/build-native-module.sh"
	xberg_assert_status 7
	[[ "$output" != *'Found 1 .node file(s)'* ]]
	xberg_assert_file crates/xberg-node/artifacts/xberg.node 'stale payload'
}

@test "Node TypeScript generation should stage both files produced by the actual wrapper invocation" {
	export EMIT_TYPES=true
	run bash "$REPOSITORY_ROOT/scripts/publish/node/generate-typescript-defs.sh"
	xberg_assert_status 0
	xberg_assert_trace '--filter {./crates/xberg-node} exec napi build --platform --dts index.d.ts'
	xberg_assert_file typescript-defs/index.d.ts 'type declarations'
	xberg_assert_file typescript-defs/index.js 'JavaScript loader'
}

@test "Node TypeScript generation should reject missing declarations after a success-looking invocation" {
	run bash "$REPOSITORY_ROOT/scripts/publish/node/generate-typescript-defs.sh"
	[ "$status" -ne 0 ]
	[ ! -e typescript-defs/index.d.ts ]
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "Node TypeScript generation should preserve a failed build before staging any files" {
	export PNPM_STATUS=9
	run bash "$REPOSITORY_ROOT/scripts/publish/node/generate-typescript-defs.sh"
	xberg_assert_status 9
	[ ! -e typescript-defs ]
}
