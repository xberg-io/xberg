#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	unset TARGET MOCK_SERVER_URL MOCK_SERVERS XBERG_TEST_DOCUMENTS_DIR BUILD_STATUS MISSING_URL
	xberg_stub_trace cargo
}

@test "CLI build should forward an explicit target with all features" {
	run bash "$REPOSITORY_ROOT/scripts/build_cli_binary.sh" aarch64-unknown-linux-gnu
	xberg_assert_status 0
	xberg_assert_trace 'cargo build --release --target aarch64-unknown-linux-gnu --package xberg-cli --features all'
}

@test "CLI build should honor the environment target and prefer an explicit argument" {
	run env TARGET=x86_64-pc-windows-msvc bash "$REPOSITORY_ROOT/scripts/build_cli_binary.sh"
	xberg_assert_status 0
	xberg_assert_trace 'cargo build --release --target x86_64-pc-windows-msvc --package xberg-cli --features all'
	: >"$XBERG_TRACE"
	run env TARGET=x86_64-pc-windows-msvc bash "$REPOSITORY_ROOT/scripts/build_cli_binary.sh" aarch64-apple-darwin
	xberg_assert_status 0
	xberg_assert_trace 'cargo build --release --target aarch64-apple-darwin --package xberg-cli --features all'
}

@test "CLI build should reject a missing target before invoking the compiler" {
	run bash "$REPOSITORY_ROOT/scripts/build_cli_binary.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Usage:'
	xberg_assert_trace_empty
}

@test "mock-server build should select exactly the requested manifests" {
	run bash "$REPOSITORY_ROOT/scripts/e2e/build-mock-server.sh" all
	xberg_assert_status 0
	xberg_assert_trace "cargo build --release --manifest-path $REPOSITORY_ROOT/e2e/rust/Cargo.toml --bin mock-server
cargo build --release --manifest-path $REPOSITORY_ROOT/test_apps/rust/Cargo.toml --bin mock-server"
	: >"$XBERG_TRACE"
	run bash "$REPOSITORY_ROOT/scripts/e2e/build-mock-server.sh"
	xberg_assert_status 0
	xberg_assert_trace "cargo build --release --manifest-path $REPOSITORY_ROOT/e2e/rust/Cargo.toml --bin mock-server"
}

@test "mock-server build should reject invalid targets and propagate the first build failure" {
	run bash "$REPOSITORY_ROOT/scripts/e2e/build-mock-server.sh" unknown
	xberg_assert_status 2
	xberg_assert_output_contains 'unknown mock-server target: unknown'
	xberg_assert_trace_empty
	xberg_stub cargo 'printf "%s\n" "$*" >>"$XBERG_TRACE"' 'exit 7'
	run bash "$REPOSITORY_ROOT/scripts/e2e/build-mock-server.sh" all
	xberg_assert_status 7
	xberg_assert_trace "build --release --manifest-path $REPOSITORY_ROOT/e2e/rust/Cargo.toml --bin mock-server"
}

prepare_mock_runner() {
	FIXTURE_ROOT="$XBERG_WORK/fixture repository"
	mkdir -p "$FIXTURE_ROOT/scripts/e2e" "$FIXTURE_ROOT/e2e/rust/target/release" "$FIXTURE_ROOT/fixtures"
	cp "$REPOSITORY_ROOT/scripts/e2e/run-with-mock-server.sh" "$FIXTURE_ROOT/scripts/e2e/"
	printf '#!/usr/bin/env bash\nprintf "build %%s\\n" "$*" >>"$XBERG_TRACE"\nexit "${BUILD_STATUS:-0}"\n' \
		>"$FIXTURE_ROOT/scripts/e2e/build-mock-server.sh"
	chmod +x "$FIXTURE_ROOT/scripts/e2e/build-mock-server.sh"
	export SERVER_PID_FILE="$BATS_TEST_TMPDIR/server-pid"
	cat >"$FIXTURE_ROOT/e2e/rust/target/release/mock-server" <<'SERVER'
#!/usr/bin/env bash
printf '%s\n' "$$" >"$SERVER_PID_FILE"
trap 'exit 0' TERM
if [ "${MISSING_URL:-false}" != true ]; then
    printf 'MOCK_SERVER_URL=http://127.0.0.1:14243\n'
fi
printf 'MOCK_SERVERS={"sample":"http://127.0.0.1:14244"}\n'
while :; do IFS= read -r -t 1 line || :; done
SERVER
	chmod +x "$FIXTURE_ROOT/e2e/rust/target/release/mock-server"
}

@test "mock runner should export handshake URLs and propagate the command status while cleaning up" {
	prepare_mock_runner
	export ENV_RECEIPT="$BATS_TEST_TMPDIR/env-receipt"
	run bash "$FIXTURE_ROOT/scripts/e2e/run-with-mock-server.sh" bash -c \
		'printf "%s\n%s\n%s\n%s\n" "$MOCK_SERVER_URL" "$MOCK_SERVERS" "$MOCK_SERVER_SAMPLE" "$XBERG_TEST_DOCUMENTS_DIR" >"$ENV_RECEIPT"; exit 6'
	xberg_assert_status 6
	xberg_assert_trace 'build e2e'
	xberg_assert_file "$ENV_RECEIPT" "http://127.0.0.1:14243
{\"sample\":\"http://127.0.0.1:14244\"}
http://127.0.0.1:14244
$FIXTURE_ROOT/test_documents"
	[ -s "$SERVER_PID_FILE" ]
	! kill -0 "$(cat "$SERVER_PID_FILE")" 2>/dev/null
}

@test "mock runner should reject an incomplete handshake without running the requested command" {
	prepare_mock_runner
	run env MISSING_URL=true bash "$FIXTURE_ROOT/scripts/e2e/run-with-mock-server.sh" touch should-not-exist
	xberg_assert_status 1
	xberg_assert_output_contains 'mock-server did not emit MOCK_SERVER_URL'
	[ ! -e should-not-exist ]
	! kill -0 "$(cat "$SERVER_PID_FILE")" 2>/dev/null
}

@test "mock runner should reject missing commands and stop before startup when its build fails" {
	prepare_mock_runner
	run bash "$FIXTURE_ROOT/scripts/e2e/run-with-mock-server.sh"
	xberg_assert_status 2
	xberg_assert_output_contains '<command> [args...]'
	xberg_assert_trace_empty
	run env BUILD_STATUS=8 bash "$FIXTURE_ROOT/scripts/e2e/run-with-mock-server.sh" touch should-not-exist
	xberg_assert_status 8
	[ ! -e "$SERVER_PID_FILE" ]
	[ ! -e should-not-exist ]
}
