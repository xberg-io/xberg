#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	WRAPPER="$REPOSITORY_ROOT/tools/benchmark-harness/scripts/liteparse_extract.sh"
	INPUT="$XBERG_WORK/input with spaces.pdf"
	printf 'fixture' >"$INPUT"
	export LIT_CONTENT='extracted "text"'
	export LIT_STATUS=0
	export CLOCK_FILE="$XBERG_WORK/clock"
	xberg_stub timeout 'shift' 'exec "$@"'
	xberg_stub date \
		'if [ -f "$CLOCK_FILE" ]; then echo 1042000000; else echo 1000000000; fi' \
		': >"$CLOCK_FILE"'
	xberg_stub lit \
		'printf "<%s>" "$@" >>"$XBERG_TRACE"' \
		'printf "\n" >>"$XBERG_TRACE"' \
		'if [ "$LIT_STATUS" -ne 0 ]; then echo "native liteparse failure" >&2; exit "$LIT_STATUS"; fi' \
		'printf "%s" "$LIT_CONTENT"'
}

@test "LiteParse wrapper should preserve native failure status and never emit a result payload" {
	export LIT_STATUS=42
	run --separate-stderr bash "$WRAPPER" --format=plaintext "$INPUT"
	xberg_assert_status 42
	xberg_assert_no_output
	[[ "$stderr" == *"native liteparse failure"* ]]
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
}

@test "LiteParse wrapper should emit valid plaintext metadata and preserve the quoted file argument" {
	run bash "$WRAPPER" --format=plaintext "$INPUT"
	xberg_assert_status 0
	xberg_assert_trace "<parse><$INPUT><--format><text><--no-links><--ocr-backend><tesseract><--quiet>"
	run python3 -c 'import json,sys; value=json.loads(sys.argv[1]); assert value == {"content":sys.argv[2],"metadata":{"framework":"liteparse","output_format":"plaintext"},"_extraction_time_ms":42}' "$output" "$LIT_CONTENT"
	xberg_assert_status 0
}

@test "LiteParse wrapper should select Markdown and forward an explicit no OCR request" {
	run bash "$WRAPPER" --format=markdown --no-ocr "$INPUT"
	xberg_assert_status 0
	xberg_assert_trace "<parse><$INPUT><--format><markdown><--ocr-backend><tesseract><--no-ocr><--quiet>"
	run python3 -c 'import json,sys; value=json.loads(sys.argv[1]); assert value["content"] == sys.argv[2]; assert value["metadata"]["output_format"] == "markdown"' "$output" "$LIT_CONTENT"
	xberg_assert_status 0
}

@test "LiteParse wrapper should reject unsupported formats before invoking the parser" {
	run bash "$WRAPPER" --format=html "$INPUT"
	xberg_assert_status 64
	xberg_assert_output_contains "unsupported format 'html'"
	xberg_assert_trace_empty
}

@test "LiteParse wrapper should reject a missing file before invoking the parser" {
	run bash "$WRAPPER" "$XBERG_WORK/missing.pdf"
	xberg_assert_status 1
	xberg_assert_output_contains 'File not found:'
	xberg_assert_trace_empty
}

@test "LiteParse wrapper should report usage when no file is provided" {
	run bash "$WRAPPER"
	xberg_assert_status 1
	xberg_assert_output_contains 'Usage:'
	xberg_assert_trace_empty
}
