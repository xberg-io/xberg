#!/usr/bin/env bats

setup_file() {
	bats_load_library xberg-bats
	xberg_shadow_system_path_without sha256sum
}

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p temporary tessdata
	export TMPDIR="$XBERG_WORK/temporary"
	unset WRONG_TOKENS ACCEPT_INVALID CHECKSUM_FAILURE DOWNLOAD_FAILURE OCR_FAILURE
	xberg_stub curl \
		'printf "curl %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${DOWNLOAD_FAILURE:-0}" = 0 ] || exit "$DOWNLOAD_FAILURE"' \
		'while [ "$1" != -o ]; do shift; done; target="$2"' \
		'case "$target" in' \
		'*.gold.txt) for ((i=0;i<60;i++)); do printf "token%s " "$i"; done >"$target" ;;' \
		'*.tif) printf "synthetic image\n" >"$target" ;;' \
		'*/LICENSE) printf "synthetic license\n" >"$target" ;;' \
		'*) exit 98;; esac'
	xberg_stub synthetic-ocr \
		'printf "ocr %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1" in */invalid.tif) [ "${ACCEPT_INVALID:-false}" = true ]; exit $?;; esac' \
		'[ "${OCR_FAILURE:-0}" = 0 ] || exit "$OCR_FAILURE"' \
		'if [ "${WRONG_TOKENS:-false}" = same_count ]; then for ((i=59;i>=0;i--)); do printf "token%s " "$i"; done; elif [ "${WRONG_TOKENS:-false}" = true ]; then printf "wrong words\n"; else for ((i=0;i<60;i++)); do printf "token%s " "$i"; done; fi'
	cat >"$XBERG_STUB_BIN/sha256sum" <<'SH'
#!/usr/bin/env bash
printf 'checksum %s\n' "$*" >>"$XBERG_TRACE"
[ "${CHECKSUM_FAILURE:-0}" = 0 ] || exit "$CHECKSUM_FAILURE"
while read -r digest filename; do
 case "$filename" in
 */phototest.tif) expected=d2241a1eb6d6cd2eb6544b8e228c2862c852b7467d16ed636caa32179b780692; payload='synthetic image';;
 */phototest.gold.txt) expected=76ef157baeb6ab275b7d90f310477a8060d5822aea2b2e05d480f459b016a6ed; payload='token0 ';;
 */LICENSE) expected=c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4; payload='synthetic license';;
 *) exit 97;; esac
 [ "$digest" = "$expected" ] || exit 96
 grep -qF "$payload" "$filename" || exit 95
 printf 'verified %s\n' "${filename##*/}" >>"$XBERG_TRACE"
done
SH
	chmod +x "$XBERG_STUB_BIN/sha256sum"
}

run_fixture() {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/verify-ocr-fixture.sh" "$XBERG_STUB_BIN/synthetic-ocr" "$XBERG_WORK/tessdata"
}

@test "OCR fixture should check all pinned checksums then compare exactly sixty tokens and reject malformed input" {
	run_fixture
	xberg_assert_status 0
	xberg_assert_output_contains '"expected_inputs": 1, "actual_ocr_inputs": 1, "expected_tokens": 60, "actual_tokens": 60'
	xberg_assert_output_contains 'Malformed OCR input rejected'
	[ "$(grep -c '^curl ' "$XBERG_TRACE")" -eq 3 ]
	base=https://raw.githubusercontent.com/tesseract-ocr/test/232ff181c66516116ec0e84c4963f70de15050fd
	for remote in testing/phototest.tif testing/phototest.gold.txt LICENSE; do
		[ "$(grep -Fc "$base/$remote" "$XBERG_TRACE")" -eq 1 ]
	done
	[ "$(grep -c '^curl --retry 3 --max-time 120 -fsSL -o ' "$XBERG_TRACE")" -eq 3 ]

	[ "$(grep -c '^verified ' "$XBERG_TRACE")" -eq 3 ]
	[ "$(grep -c '^checksum ' "$XBERG_TRACE")" -eq 2 ]
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 2 ]
	grep -F -- "--tessdata-dir $XBERG_WORK/tessdata -l eng --psm 3" "$XBERG_TRACE"
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should use shasum fallback when sha256sum is unavailable" {
	mv "$XBERG_STUB_BIN/sha256sum" "$XBERG_STUB_BIN/shasum"
	export PATH="$XBERG_STUB_BIN:$XBERG_SYS_BIN"
	run_fixture
	xberg_assert_status 0
	[ "$(grep -c '^checksum -a 256 -c -$' "$XBERG_TRACE")" -eq 2 ]
	[ "$(grep -c '^verified ' "$XBERG_TRACE")" -eq 3 ]
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 2 ]
}

@test "OCR fixture should stop on failed checksums before license download or OCR" {
	export CHECKSUM_FAILURE=4
	run_fixture
	xberg_assert_status 4
	[ "$(grep -c '^curl ' "$XBERG_TRACE")" -eq 2 ]
	[ "$(grep -c '^checksum ' "$XBERG_TRACE")" -eq 1 ]
	run grep -q '^ocr ' "$XBERG_TRACE"
	xberg_assert_status 1
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should fail when output has fewer than sixty matching tokens" {
	export WRONG_TOKENS=true
	run_fixture
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 60 exact tokens, received 2'
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should fail when selected binary accepts malformed input" {
	export ACCEPT_INVALID=true
	run_fixture
	xberg_assert_status 1
	xberg_assert_output_contains 'Malformed image negative control was accepted'
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 2 ]
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should propagate extraction failure without running malformed input" {
	export OCR_FAILURE=7
	run_fixture
	xberg_assert_status 7
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should propagate failed download before checksum verification or OCR" {
	export DOWNLOAD_FAILURE=22
	run_fixture
	xberg_assert_status 22
	[ "$(grep -c '^curl ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(wc -l <"$XBERG_TRACE")" -eq 1 ]
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}

@test "OCR fixture should reject sixty tokens in the wrong order" {
	export WRONG_TOKENS=same_count
	run_fixture
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 60 exact tokens, received 60'
	[ "$(grep -c '^ocr ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(find temporary -mindepth 1 | wc -l)" -eq 0 ]
}
