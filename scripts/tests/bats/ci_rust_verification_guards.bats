#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export RUNTIME_TEST='api::live_engine_tests::clear_reaches_the_linked_native_implementation_without_initialization'
	export AUTO_TEST='layout::engine::auto_gate_vlm_region_tests::auto_gated_native_pdf_keeps_detections_for_vlm_regions'
	unset CARGO_FAILURE RUNTIME_COUNT RUNTIME_NAME EXTRA_LOG RESOLVED_FEATURES LIST_COUNT RUN_COUNT
}

stub_dynamic_cargo() {
	xberg_stub cargo \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'printf "%s\n" "Using dynamic linking with system-installed Tesseract libraries"' \
		'printf "test %s ... ok\n" "${RUNTIME_NAME:-$RUNTIME_TEST}"' \
		'printf "test result: ok. %s passed; 0 failed; 0 ignored; 0 measured;\n" "${RUNTIME_COUNT:-1}"' \
		'printf "%s\n" "${EXTRA_LOG:-}"' \
		'exit "${CARGO_FAILURE:-0}"'
}

@test "dynamic Tesseract verification should run the exact linked implementation once" {
	stub_dynamic_cargo
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/check-tesseract-dynamic.sh"
	xberg_assert_status 0
	xberg_assert_trace "test --locked -p xberg-tesseract --no-default-features --features dynamic-linking --lib $RUNTIME_TEST -vv -- --exact"
	xberg_assert_output_contains '(1 passed)'
}

@test "dynamic Tesseract verification should reject a successful cargo command that ran zero tests" {
	stub_dynamic_cargo
	export RUNTIME_COUNT=0
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/check-tesseract-dynamic.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'runtime smoke test did not execute exactly once'
}

@test "dynamic Tesseract verification should reject a different passing test" {
	stub_dynamic_cargo
	export RUNTIME_NAME=unrelated_test
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/check-tesseract-dynamic.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'runtime smoke test did not execute exactly once'
}

@test "dynamic Tesseract verification should reject vendored download activity" {
	stub_dynamic_cargo
	export EXTRA_LOG='Downloading Tesseract from https://example.invalid/archive'
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/check-tesseract-dynamic.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'unexpectedly entered the vendored build path'
}

@test "dynamic Tesseract verification should propagate cargo failure before accepting its log" {
	stub_dynamic_cargo
	export CARGO_FAILURE=7
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/check-tesseract-dynamic.sh"
	xberg_assert_status 1
	[[ "$output" != *'Verified host Tesseract'* ]]
}

stub_auto_cargo() {
	xberg_stub cargo \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'if [ "$1" = tree ]; then printf "%s\n" "${RESOLVED_FEATURES-tokio-runtime,pdf,ocr,tesseract-dynamic,layout-detection,liter-llm}"; exit 0; fi' \
		'case " $* " in *" --list "*) for ((i=0;i<${LIST_COUNT:-1};i++)); do printf "%s: test\n" "$AUTO_TEST"; done ;;' \
		'*) printf "test %s ... ok\n" "$AUTO_TEST"; printf "test result: ok. %s passed; 0 failed; 0 ignored;\n" "${RUN_COUNT:-1}" ;; esac'
}

@test "VLM auto gate should verify resolved features and run exactly one listed regression" {
	stub_auto_cargo
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'expected 1, listed 1, passed 1'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 3 ]
	grep -Fq -- "--lib $AUTO_TEST -- --exact --list" "$XBERG_TRACE"
	grep -Fq -- "--lib $AUTO_TEST -- --exact --test-threads=1" "$XBERG_TRACE"
}

@test "VLM auto gate should reject an empty resolved feature graph before testing" {
	stub_auto_cargo
	export RESOLVED_FEATURES=''
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'resolved feature list is empty'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
}

@test "VLM auto gate should reject accidentally enabled formula recognition" {
	stub_auto_cargo
	export RESOLVED_FEATURES='tokio-runtime,pdf,ocr,tesseract-dynamic,layout-detection,liter-llm,formula-recognition'
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'must run without formula-recognition'
}

@test "VLM auto gate should reject a missing required feature" {
	stub_auto_cargo
	export RESOLVED_FEATURES='tokio-runtime,pdf,ocr,tesseract-dynamic,layout-detection'
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'required resolved feature liter-llm is absent'
}

@test "VLM auto gate should reject a successful listing containing zero tests" {
	stub_auto_cargo
	export LIST_COUNT=0
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one exact regression test, listed 0'
}

@test "VLM auto gate should reject duplicate collected regressions" {
	stub_auto_cargo
	export LIST_COUNT=2
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one exact regression test, listed 2'
}

@test "VLM auto gate should reject a collected regression that was not executed" {
	stub_auto_cargo
	export RUN_COUNT=0
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-vlm-auto-gate-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one passing regression test with none ignored'
}

stub_mime_cargo() {
	xberg_stub cargo \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'expected=11; case "$*" in *core::extractor::bytes*) expected=4; case "$*" in *tower-service*) expected=6 ;; esac ;; esac' \
		'case " $* " in *" --list "*) for ((i=0;i<${LIST_COUNT:-$expected};i++)); do printf "regression_%s: test\n" "$i"; done ;;' \
		'*) printf "test result: ok. %s passed; 0 failed; 0 ignored;\n" "${RUN_COUNT:-$expected}" ;; esac'
}

@test "MIME verification should collect and execute both feature graphs with all 32 regressions" {
	stub_mime_cargo
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-mime-tests.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'configurations expected 2, observed 2; tests expected 32, passed 32'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 8 ]
	[ "$(grep -c -- '--features tokio-runtime,tower-service' "$XBERG_TRACE")" -eq 4 ]
	[ "$(grep -c -- '--list' "$XBERG_TRACE")" -eq 4 ]
}

@test "MIME verification should reject an empty collected group before running it" {
	stub_mime_cargo
	export LIST_COUNT=0
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-mime-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 4 tests, listed 0'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
}

@test "MIME verification should reject a collected group that executes zero regressions" {
	stub_mime_cargo
	export RUN_COUNT=0
	run bash "$REPOSITORY_ROOT/scripts/ci/rust/run-mime-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 4 passing tests with none ignored'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
}
