#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SMOKE="$REPOSITORY_ROOT/scripts/ci/smoke-cli-ort.sh"
	STAGE="$BATS_TEST_TMPDIR/xberg-cli-stage"
	mkdir -p "$STAGE"
	: >"$STAGE/libonnxruntime.so"
	# The overrides the smoke must clear; the fake CLI fails if it sees any of them. ~keep
	export ORT_DYLIB_PATH=/ambient/libonnxruntime.so LD_LIBRARY_PATH=/ambient DYLD_LIBRARY_PATH=/ambient
	export DYLD_FALLBACK_LIBRARY_PATH=/ambient
}

# A stand-in for the staged CLI: it "loads" ONNX Runtime only from its own directory, as the
# strict resolver does, and behaves as `MODE` says. ~keep
fake_cli() {
	local mode="$1"
	cat >"$STAGE/xberg" <<EOF
#!/usr/bin/env bash
set -euo pipefail
for variable in ORT_DYLIB_PATH LD_LIBRARY_PATH DYLD_LIBRARY_PATH DYLD_FALLBACK_LIBRARY_PATH; do
	if [ -n "\${!variable:-}" ]; then echo "leaked \$variable" >&2; exit 7; fi
done
dir="\$(cd "\$(dirname "\$0")" && pwd -P)"
mode="$mode"
if [ -e "\$dir/libonnxruntime.so" ] || [ "\$mode" = ignores-library ]; then
	if [ "\$mode" = broken ]; then echo "session failed" >&2; exit 3; fi
	echo "DEBUG xberg::ort_discovery: Loaded ONNX Runtime path=\$dir/libonnxruntime.so" >&2
	echo '{"embeddings": [[0.1, 0.2]], "model": "fast", "dimensions": 2, "count": 1}'
	exit 0
fi
if [ "\$mode" = other-failure ]; then echo "dyld: Library not loaded" >&2; exit 4; fi
echo "Error: ONNX Runtime library not found; searched: \$dir/libonnxruntime.so" >&2
exit 1
EOF
	chmod +x "$STAGE/xberg"
}

@test "ORT smoke should pass when the staged runtime builds a session and its absence fails in the resolver" {
	fake_cli good

	run "$SMOKE" "$STAGE"

	xberg_assert_status 0
	xberg_assert_output_contains "built an ONNX Runtime session through the runtime shipped in"
}

@test "ORT smoke should fail when the staged CLI cannot build a session" {
	fake_cli broken

	run "$SMOKE" "$STAGE"

	xberg_assert_status 1
	xberg_assert_output_contains "could not build an ONNX Runtime session"
}

@test "ORT smoke should fail when the embedding succeeds without the shipped runtime" {
	fake_cli ignores-library

	run "$SMOKE" "$STAGE"

	xberg_assert_status 1
	xberg_assert_output_contains "the check cannot tell the shipped library apart"
}

@test "ORT smoke should fail when the missing runtime fails for another reason" {
	fake_cli other-failure

	run "$SMOKE" "$STAGE"

	xberg_assert_status 1
	xberg_assert_output_contains "failed for another reason than the library search"
}

@test "ORT smoke should refuse a stage without a runtime" {
	fake_cli good
	rm "$STAGE/libonnxruntime.so"

	run "$SMOKE" "$STAGE"

	xberg_assert_status 1
	xberg_assert_output_contains "no ONNX Runtime library staged"
}
