#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export FIXTURE_ROOT="$XBERG_WORK/fixture repository"
	mkdir -p "$FIXTURE_ROOT/scripts/ci/fixtures/csharp-smoke" "$FIXTURE_ROOT/scripts/ci/fixtures/java-smoke" \
		"$FIXTURE_ROOT/packages/java/io/xberg" "$FIXTURE_ROOT/packages/elixir/lib/xberg" "$FIXTURE_ROOT/crates/xberg-node" \
		"$XBERG_WORK/native"
	cp "$REPOSITORY_ROOT"/scripts/ci/smoke-test-musl-*.sh "$REPOSITORY_ROOT/scripts/ci/smoke-test-php-debian12.sh" "$FIXTURE_ROOT/scripts/ci/"
	for file in scripts/ci/fixtures/musl-python-smoke.pdf scripts/ci/fixtures/csharp-smoke/Smoke.csproj \
		scripts/ci/fixtures/java-smoke/Smoke.java packages/java/pom.xml packages/elixir/lib/xberg/native.ex \
		scripts/ci/smoke_test_elixir_nif.exs scripts/ci/smoke_test_node_binding.js scripts/ci/smoke_test_python_wheel.py crates/xberg-node/index.js; do
		printf 'fixture bytes\n' >"$FIXTURE_ROOT/$file"
	done
	printf 'native bytes\n' >"$XBERG_WORK/native/libxberg_ffi.so"
	printf 'nif bytes\n' >"$XBERG_WORK/native/libxberg_nif.so"
	printf 'node bytes\n' >"$XBERG_WORK/native/xberg.node"
	printf 'wheel bytes\n' >"$XBERG_WORK/xberg.whl"
	printf 'php bytes\n' >"$XBERG_WORK/xberg.tgz"
	unset DOCKER_STATUS
	xberg_stub docker 'printf "%s\n" "$@" >"$XBERG_TRACE"' 'exit "${DOCKER_STATUS:-0}"'
	cd "$XBERG_WORK" || return
}

run_native_smoke() {
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-$1.sh" native
}

assert_container() {
	[ "$(grep -c '^run$' "$XBERG_TRACE")" -eq 1 ]
	grep -Fxq "$1" "$XBERG_TRACE"
	grep -Fxq 'EXPECTED_TEXT=XBERG MUSL SMOKE 490' "$XBERG_TRACE"
	grep -Fxq "$FIXTURE_ROOT/scripts/ci/fixtures/musl-python-smoke.pdf:/fixture.pdf:ro" "$XBERG_TRACE"
	grep -Fq "$2" "$XBERG_TRACE"
}

@test "musl Csharp smoke should mount the absolute closure and execute the extraction fixture" {
	run_native_smoke csharp-ffi
	xberg_assert_status 0
	assert_container mcr.microsoft.com/dotnet/sdk:10.0-alpine 'dotnet exec /out/Smoke.dll /fixture.pdf'
	grep -Fxq "$XBERG_WORK/native:/native:ro" "$XBERG_TRACE"
	grep -Fq 'dotnet build -c Release' "$XBERG_TRACE"
}

@test "musl Elixir smoke should mount generated native exports and the complete closure" {
	run_native_smoke elixir-nif
	xberg_assert_status 0
	assert_container alpine:3.21 'elixir /smoke/smoke_test_elixir_nif.exs /fixture.pdf'
	grep -Fxq "$XBERG_WORK/native:/native:ro" "$XBERG_TRACE"
	grep -Fxq "$FIXTURE_ROOT/packages/elixir/lib/xberg/native.ex:/smoke/native.ex:ro" "$XBERG_TRACE"
	grep -Fxq 'XBERG_NIF_PATH=/native/libxberg_nif' "$XBERG_TRACE"
	grep -Fq 'apk add --no-cache --upgrade libstdc++' "$XBERG_TRACE"
}

@test "musl Java smoke should compile actual package sources and invoke the extraction program" {
	run_native_smoke java-ffi
	xberg_assert_status 0
	assert_container alpine:3.21 'Smoke /fixture.pdf'
	grep -Fxq "$XBERG_WORK/native:/native:ro" "$XBERG_TRACE"
	grep -Fxq 'XBERG_FFI_LIB_PATH=/native/libxberg_ffi.so' "$XBERG_TRACE"
	grep -Fq 'javac --release 25 --enable-preview' "$XBERG_TRACE"
	grep -Fq 'dependency:build-classpath' "$XBERG_TRACE"
}

@test "musl Node smoke should select the actual addon and run extraction rather than only require" {
	run_native_smoke node-binding
	xberg_assert_status 0
	assert_container alpine:3.22 'node /smoke/smoke_test_node_binding.js /smoke/index.js /fixture.pdf'
	grep -Fxq 'NAPI_RS_NATIVE_LIBRARY_PATH=/native/xberg.node' "$XBERG_TRACE"
	grep -Fxq "$XBERG_WORK/native:/native:ro" "$XBERG_TRACE"
	grep -Fq 'apk add --no-cache --upgrade libstdc++' "$XBERG_TRACE"
}

@test "musl Python smoke should mount the selected wheel and invoke the extraction script" {
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-python-wheel.sh" xberg.whl
	xberg_assert_status 0
	assert_container alpine:3.22 '/venv/bin/python3 /smoke_test_python_wheel.py /fixture.pdf'
	grep -Fxq "$XBERG_WORK/xberg.whl:/wheel/xberg.whl:ro" "$XBERG_TRACE"
	grep -Fxq 'WHEEL_NAME=xberg.whl' "$XBERG_TRACE"
	grep -Fq '/venv/bin/pip install --no-cache-dir --quiet "/wheel/${WHEEL_NAME}"' "$XBERG_TRACE"
}

@test "Debian PHP smoke should preserve its requested version and validate the unpacked native closure" {
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-php-debian12.sh" xberg.tgz 8.5
	xberg_assert_status 0
	grep -Fxq 'php:8.5-cli-bookworm' "$XBERG_TRACE"
	grep -Fxq "$XBERG_WORK/xberg.tgz:/tmp/xberg.tgz:ro" "$XBERG_TRACE"
	grep -Fq '[ "$extension_count" = 1 ]' "$XBERG_TRACE"
	grep -Fq 'ldd_report="$(ldd "$extension" 2>&1)"' "$XBERG_TRACE"
	grep -Fq '"$extension_dir"/libonnxruntime.so*)' "$XBERG_TRACE"
	grep -Fq 'exit "$status"' "$XBERG_TRACE"
	grep -Fq 'php -n -d "extension=$extension" --ri xberg' "$XBERG_TRACE"
}

@test "container smoke wrappers should propagate actual container failures for all six runtimes" {
	export DOCKER_STATUS=23
	for variant in csharp-ffi elixir-nif java-ffi node-binding; do
		run_native_smoke "$variant"
		xberg_assert_status 23
		grep -Fxq 'run' "$XBERG_TRACE"
	done
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-python-wheel.sh" xberg.whl
	xberg_assert_status 23
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-php-debian12.sh" xberg.tgz 8.5
	xberg_assert_status 23
}

@test "native container smoke wrappers should reject an empty artifact directory before Docker" {
	mkdir empty
	for variant in csharp-ffi elixir-nif java-ffi node-binding; do
		run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-$variant.sh" empty
		xberg_assert_status 1
		xberg_assert_output_contains 'found under'
		xberg_assert_trace_empty
	done
}

@test "container smoke wrappers should reject absent artifacts before Docker" {
	for variant in csharp-ffi elixir-nif java-ffi node-binding python-wheel; do
		run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-$variant.sh" absent
		xberg_assert_status 1
		xberg_assert_output_contains 'not found'
		xberg_assert_trace_empty
	done
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-php-debian12.sh" absent 8.5
	xberg_assert_status 1
	xberg_assert_output_contains 'artifact does not exist'
	xberg_assert_trace_empty
}

@test "musl smoke wrappers should reject a missing extraction fixture before Docker" {
	rm "$FIXTURE_ROOT/scripts/ci/fixtures/musl-python-smoke.pdf"
	for variant in csharp-ffi elixir-nif java-ffi node-binding; do
		run_native_smoke "$variant"
		xberg_assert_status 1
		xberg_assert_output_contains 'smoke-test fixture missing'
		xberg_assert_trace_empty
	done
	run "$BASH" "$FIXTURE_ROOT/scripts/ci/smoke-test-musl-python-wheel.sh" xberg.whl
	xberg_assert_status 1
	xberg_assert_output_contains 'smoke-test fixture missing'
	xberg_assert_trace_empty
}

@test "Elixir smoke should reject absent generated exports before a misleading load failure" {
	rm "$FIXTURE_ROOT/packages/elixir/lib/xberg/native.ex"
	run_native_smoke elixir-nif
	xberg_assert_status 1
	xberg_assert_output_contains 'generated Xberg.Native module missing'
	xberg_assert_trace_empty
}

@test "Java smoke should reject a missing Maven manifest before invoking Docker" {
	rm "$FIXTURE_ROOT/packages/java/pom.xml"
	run_native_smoke java-ffi
	xberg_assert_status 1
	xberg_assert_output_contains 'packages/java/pom.xml missing'
	xberg_assert_trace_empty
}

@test "Node smoke should reject a missing native loader before invoking Docker" {
	rm "$FIXTURE_ROOT/crates/xberg-node/index.js"
	run_native_smoke node-binding
	xberg_assert_status 1
	xberg_assert_output_contains 'napi-rs index.js missing'
	xberg_assert_trace_empty
}
