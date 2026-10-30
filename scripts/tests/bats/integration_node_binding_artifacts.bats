#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p crates/xberg-node
	cp "$REPOSITORY_ROOT/crates/xberg-node/package.json" crates/xberg-node/
	printf 'reviewed loader\n' >crates/xberg-node/index.js
	printf 'export wrapper\n' >crates/xberg-node/index.mjs
	printf 'binding wrapper\n' >crates/xberg-node/binding.cjs
	printf 'type declarations\n' >crates/xberg-node/index.d.ts
	unset INSTALL_STATUS BUILD_STATUS
	# The task body runs directly: under `mise run`, the pnpm mise installs would shadow the stub.
	BUILD_TASK="$REPOSITORY_ROOT/mise-tasks/integrations/ci/build/node-binding.sh"
	xberg_stub pnpm \
		'printf "%s|jobs=%s\n" "$*" "${CARGO_BUILD_JOBS:-unset}" >>"$XBERG_TRACE"' \
		'if [ "$1" = install ]; then exit "${INSTALL_STATUS:-0}"; fi' \
		'case " $* " in *" --no-js "*) ;; *) printf "unreviewed generated loader\n" >index.js ;; esac' \
		'printf "native artifact\n" >fixture.node' \
		'exit "${BUILD_STATUS:-0}"'
}

@test "integration Node binding build should stage every exported entry point and preserve the reviewed loader" {
	run "$BASH" "$BUILD_TASK"
	xberg_assert_status 0
	xberg_assert_trace 'install|jobs=unset
exec napi build --no-js --output-dir . --release --platform|jobs=1'
	xberg_assert_file crates/xberg-node/index.js 'reviewed loader'
	xberg_assert_file dist/node/index.js 'reviewed loader'
	xberg_assert_file dist/node/index.mjs 'export wrapper'
	xberg_assert_file dist/node/binding.cjs 'binding wrapper'
	xberg_assert_file dist/node/index.d.ts 'type declarations'
	xberg_assert_file dist/node/fixture.node 'native artifact'
	cmp crates/xberg-node/package.json dist/node/package.json
	[ "$(find dist/node -type f | wc -l | tr -d '[:space:]')" -eq 6 ]
}

@test "integration Node binding build should stop before compilation when installation fails" {
	export INSTALL_STATUS=7
	run "$BASH" "$BUILD_TASK"
	[ "$status" -ne 0 ]
	xberg_assert_trace 'install|jobs=unset'
	[ ! -e dist/node ]
}

@test "integration Node binding build should reject a failed compiler despite an emitted native artifact" {
	export BUILD_STATUS=9
	run "$BASH" "$BUILD_TASK"
	[ "$status" -ne 0 ]
	[ -f crates/xberg-node/fixture.node ]
	[ ! -e dist/node ]
}
