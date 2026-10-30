#!/usr/bin/env bats

setup_file() {
	bats_load_library xberg-bats
	xberg_shadow_system_path_without wasm-pack
}

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	PATH="$(xberg_isolated_path)"
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export FIXTURE_ROOT="$XBERG_WORK/repository"
	mkdir -p "$FIXTURE_ROOT/scripts/ci/wasm" "$XBERG_WORK/home/.cargo/bin"
	cp "$REPOSITORY_ROOT/scripts/ci/wasm/run-crate-tests.sh" "$REPOSITORY_ROOT/scripts/ci/wasm/install-wasm-pack.sh" \
		"$FIXTURE_ROOT/scripts/ci/wasm/"
	unset CC_STATUS POSIX_STATUS WASM_STATUS
	xberg_stub cargo 'printf "cargo %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${INSTALL_STATUS:-0}"'
	xberg_stub curl 'printf "curl %s\n" "$*" >>"$XBERG_TRACE"' 'printf "exit %s\\n" "${INSTALL_STATUS:-0}"'
	xberg_stub uname 'printf "%s\n" "${TEST_OS:-Linux}"'
}

stub_wasm_pack() {
	xberg_stub wasm-pack \
		'printf "wasm-pack %s\n" "$*" >>"$XBERG_TRACE"' \
		'printf "flags=%s encoded=%s node=%s\n" "${RUSTFLAGS-unset}" "${CARGO_ENCODED_RUSTFLAGS-unset}" "${NODE_PATH:-}" >>"$XBERG_TRACE"' \
		'exit "${WASM_STATUS:-0}"'
}

stub_posix_compiler() {
	xberg_stub cc \
		'printf "cc %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CC_STATUS:-0}" = 0 ] || exit "$CC_STATUS"' \
		'while [ "$1" != -o ]; do shift; done' \
		'printf '\''#!/usr/bin/env bash\nprintf "posix probe\\n" >>"$XBERG_TRACE"\nexit "${POSIX_STATUS:-0}"\n'\'' >"$2"' \
		'chmod +x "$2"'
}

refute_trace_pattern() {
	[ -f "$XBERG_TRACE" ]
	[ "$(grep -Ec -- "$1" "$XBERG_TRACE" || true)" -eq 0 ]
}

@test "WASM installer should use an existing wasm-pack without downloading or changing workflow PATH" {
	stub_wasm_pack
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/install-wasm-pack.sh"
	xberg_assert_status 0
	grep -Fxq 'wasm-pack --version' "$XBERG_TRACE"
	[ "$(grep -c '^wasm-pack ' "$XBERG_TRACE")" -eq 1 ]
	refute_trace_pattern '^(cargo|curl) '
	xberg_assert_github_path ''
}

@test "WASM installer should install through Cargo on Windows and export the isolated Cargo bin directory" {
	export TEST_OS=MINGW64_NT
	run env HOME="$XBERG_WORK/home" "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/install-wasm-pack.sh"
	xberg_assert_status 0
	xberg_assert_trace 'cargo install wasm-pack --locked --force'
	xberg_assert_github_path "$XBERG_WORK/home/.cargo/bin"
}

@test "WASM installer should reject a failed Windows installation without exporting success paths" {
	export TEST_OS=MINGW64_NT INSTALL_STATUS=24
	run env HOME="$XBERG_WORK/home" "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/install-wasm-pack.sh"
	xberg_assert_status 24
	xberg_assert_trace 'cargo install wasm-pack --locked --force'
	xberg_assert_github_path ''
}

@test "WASM installer should execute the downloaded installer and preserve its failure" {
	export TEST_OS=Linux INSTALL_STATUS=25
	run env HOME="$XBERG_WORK/home" "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/install-wasm-pack.sh"
	xberg_assert_status 25
	xberg_assert_trace 'curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf'
	xberg_assert_github_path ''
}

@test "WASM test runner should execute the POSIX probe then remove native flags and preserve existing Node module paths" {
	stub_wasm_pack
	stub_posix_compiler
	export RUSTFLAGS='native flags' CARGO_ENCODED_RUSTFLAGS='encoded native flags' NODE_PATH=existing-shims
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/run-crate-tests.sh"
	xberg_assert_status 0
	grep -Fq -- '-Dsystem=xberg_test_system -Dmkstemp=xberg_test_mkstemp' "$XBERG_TRACE"
	grep -Fxq 'posix probe' "$XBERG_TRACE"
	grep -Fxq "wasm-pack test --node $FIXTURE_ROOT/crates/xberg-wasm" "$XBERG_TRACE"
	grep -Fxq "flags=unset encoded=unset node=$FIXTURE_ROOT/crates/xberg-wasm/test-shims:existing-shims" "$XBERG_TRACE"
	[ "$(grep -c '^wasm-pack ' "$XBERG_TRACE")" -eq 1 ]
}

@test "WASM test runner should stop before Node tests when the C probe cannot be compiled" {
	stub_wasm_pack
	stub_posix_compiler
	export CC_STATUS=26
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/run-crate-tests.sh"
	xberg_assert_status 26
	refute_trace_pattern '^(posix|wasm-pack) '
}

@test "WASM test runner should reject a failing executed POSIX probe rather than only checking the binary exists" {
	stub_wasm_pack
	stub_posix_compiler
	export POSIX_STATUS=27
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/run-crate-tests.sh"
	xberg_assert_status 27
	grep -Fxq 'posix probe' "$XBERG_TRACE"
	refute_trace_pattern '^wasm-pack '
}

@test "WASM test runner should propagate the actual wasm-pack test failure" {
	stub_wasm_pack
	stub_posix_compiler
	export WASM_STATUS=28
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/wasm/run-crate-tests.sh"
	xberg_assert_status 28
	grep -Fxq 'posix probe' "$XBERG_TRACE"
	grep -Fxq "wasm-pack test --node $FIXTURE_ROOT/crates/xberg-wasm" "$XBERG_TRACE"
}
