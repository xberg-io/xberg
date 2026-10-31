# shellcheck disable=SC2016 # Stub bodies expand when invoked. ~keep
setup() {
	bats_require_minimum_version 1.5.0
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export QUALIFIER_ROOT="$XBERG_WORK/repo" EXPECTED_HEIF_VERSION=1.19.8
	mkdir -p "$QUALIFIER_ROOT/scripts/ci/rust" "$QUALIFIER_ROOT/test_documents/images"
	cp "$REPO_ROOT/scripts/ci/rust/check-libheif-runtime.sh" "$QUALIFIER_ROOT/scripts/ci/rust/"
	python3 - "$QUALIFIER_ROOT/test_documents" <<'PY'
import hashlib
import json
import sys
from pathlib import Path
root = Path(sys.argv[1])
objects = {}
for name in ['test.heic', 'test.heif', 'test.avif', 'alpha.heif']:
    data = name.encode()
    (root / 'images' / name).write_bytes(data)
    objects['images/' + name] = {'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
(root / 'corpus.lock.json').write_text(json.dumps({'objects': objects}))
PY
	cat >"$QUALIFIER_ROOT/runtime-tests" <<'SH'
#!/usr/bin/env bash
if [ "$1" = --list ]; then
  for ((number=1; number<=${MOCK_TEST_COUNT:-5}; number++)); do echo "decode_$number: test"; done
else
  if [ "${MOCK_SKIP:-false}" = true ]; then echo 'SKIP: fixture missing'; fi
  echo 'test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'
fi
SH
	chmod +x "$QUALIFIER_ROOT/runtime-tests"
	REAL_PYTHON="$(command -v python3)"
	export REAL_PYTHON
	xberg_stub python3 \
		'if [ "$1" = scripts/ci/verify_heif_runtime.py ]; then' \
		'  printf "%s\n" "$*" >"$QUALIFIER_ROOT/python-qualifier-call"' \
		'  exit "${MOCK_PYTHON_STATUS:-0}"' \
		'fi' \
		'exec "$REAL_PYTHON" "$@"'
	xberg_stub pkg-config \
		'case "$1" in' \
		'  --modversion) echo 1.19.8 ;;' \
		'  --cflags) echo "-I/mock/include -L/usr/local/lib -lheif" ;;' \
		'  *) exit 90 ;;' \
		'esac'
	cat >"$QUALIFIER_ROOT/probe-stub" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$1" >>"$QUALIFIER_ROOT/probe-calls"
[ "${MOCK_WRONG_VERSION_ACCEPTED:-false}" = true ] && exit 0
[ "$1" = "${MOCK_RUNTIME_VERSION:-1.19.8}" ]
SH
	xberg_stub cc \
		'printf "%s\n" "$*" >"$QUALIFIER_ROOT/compiler-call"' \
		'cp "$1" "$QUALIFIER_ROOT/heif-abi.c"' \
		'[ "${MOCK_MISSING_PROBE:-false}" = true ] && exit 0' \
		'target="${!#}"' \
		'cp "$QUALIFIER_ROOT/probe-stub" "$target"' \
		'chmod +x "$target"'
	xberg_stub cargo 'printf "{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"decode_smoke\"},\"executable\":\"%s/runtime-tests\"}\n" "$QUALIFIER_ROOT"'
	xberg_stub ldd \
		'printf "%s\n" "${1##*/}" >>"$QUALIFIER_ROOT/linkage-calls"' \
		'if [ "${1##*/}" = heif-abi ]; then' \
		'  echo "libheif.so.1 => /usr/local/lib/libheif.so.1"' \
		'else' \
		'  echo "libheif.so.1 => ${MOCK_LIBRARY:-/usr/local/lib/libheif.so.1}"' \
		'fi'
	xberg_stub nm 'printf " U heif_image_get_plane_readonly\n"; if [ "${MOCK_NEW_API:-false}" = true ]; then echo " U heif_image_get_plane_readonly2"; fi'
}

@test "libheif runtime harness should_accept_five_executed_tests_and_four_verified_inputs" {
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'expected 5, passed 5; fixtures 4'
	[ "$(cat "$QUALIFIER_ROOT/probe-calls")" = $'1.19.8\n0.0.0' ]
	[ "$(cat "$QUALIFIER_ROOT/linkage-calls")" = $'heif-abi\nruntime-tests' ]
	[ "$(cat "$QUALIFIER_ROOT/python-qualifier-call")" = 'scripts/ci/verify_heif_runtime.py --library /usr/local/lib/libheif.so.1 --version 1.19.8' ]
	grep -q '#include <libheif/heif.h>' "$QUALIFIER_ROOT/heif-abi.c"
	grep -q 'heif_context_alloc()' "$QUALIFIER_ROOT/heif-abi.c"
	grep -q -- '-I/mock/include -L/usr/local/lib -lheif -Wl,-rpath,/usr/local/lib -o ' "$QUALIFIER_ROOT/compiler-call"
}

@test "libheif runtime harness should_reject_zero_collected_tests" {
	export MOCK_TEST_COUNT=0
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 5 tests, collected 0'
}

@test "libheif runtime harness should_reject_skipped_fixtures_even_with_green_summary" {
	export MOCK_SKIP=true
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'A runtime fixture test skipped'
}

@test "libheif runtime harness should_reject_post_floor_symbols" {
	export MOCK_NEW_API=true
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'post-floor plane API'
}

@test "libheif runtime harness should_reject_a_shadowed_system_library" {
	export MOCK_LIBRARY=/usr/lib/libheif.so.1
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
}

@test "libheif runtime harness should_reject_a_missing_fixture_before_building" {
	rm "$QUALIFIER_ROOT/test_documents/images/test.heic"
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'FileNotFoundError'
	xberg_assert_output_contains 'images/test.heic'
}

@test "libheif runtime harness should_reject_a_missing_compiled_C_probe" {
	export MOCK_MISSING_PROBE=true
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'C runtime probe was not produced as an executable'
	[ ! -f "$QUALIFIER_ROOT/python-qualifier-call" ]
}

@test "libheif runtime harness should_reject_a_mismatched_C_runtime" {
	export MOCK_RUNTIME_VERSION=1.23.7
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	[ "$(cat "$QUALIFIER_ROOT/probe-calls")" = '1.19.8' ]
	[ ! -f "$QUALIFIER_ROOT/python-qualifier-call" ]
}

@test "libheif runtime harness should_reject_a_C_probe_accepting_the_wrong_version" {
	export MOCK_WRONG_VERSION_ACCEPTED=true
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'wrong-runtime negative control passed'
}

@test "libheif runtime harness should_propagate_python_decoder_failure" {
	export MOCK_PYTHON_STATUS=23
	run bash "$QUALIFIER_ROOT/scripts/ci/rust/check-libheif-runtime.sh"
	xberg_assert_status 23
	[ -f "$QUALIFIER_ROOT/python-qualifier-call" ]
}
