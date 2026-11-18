#!/usr/bin/env bats

setup_file() {
	bats_load_library xberg-bats
	xberg_shadow_system_path_without sha256sum shasum
}

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	export XBERG_INSTALL="$XBERG_WORK/installed" XBERG_VERSION=1.4.0
	unset XBERG_LINUX_LIBC GITHUB_TOKEN
	export CLI_SYSTEM=Linux CLI_ARCH=arm64 CLI_TARGET=aarch64-unknown-linux-musl
	export CLI_ARCHIVE="$XBERG_WORK/cli.tar.gz"
	unset CURL_STATUS NO_RELEASE GNU_STATUS GNU_HTTP GNU_ARCHIVE CLI_DIGEST CLI_METADATA_MODE
	export CLI_AUTH_TRACE="$XBERG_WORK/authorizations"
	xberg_stub uname 'case "$1" in -s) printf "%s\n" "$CLI_SYSTEM" ;; -m) printf "%s\n" "$CLI_ARCH" ;; *) exit 97 ;; esac'
	xberg_stub ldd 'printf "musl libc\n"'
	xberg_stub release-metadata \
		'checksum="$(shasum -a 256 "$CLI_ARCHIVE")"; digest="sha256:${checksum%% *}"' \
		'printf "{\"tag_name\":\"v1.4.0\","' \
		'if [ "${CLI_METADATA_MODE:-}" = duplicate-assets ]; then printf "\"assets\":[],"; fi' \
		'printf "\"assets\":[{"' \
		'case "${CLI_METADATA_MODE:-}" in' \
		'  no-asset | duplicate-name) printf "\"name\":\"different.tar.gz\"," ;;' \
		'  duplicate-digest) printf "\"digest\":null," ;;' \
		'esac' \
		'if [ "${CLI_METADATA_MODE:-}" != no-asset ]; then printf "\"name\":\"xberg-cli-%s.tar.gz\"," "$CLI_TARGET"; fi' \
		'printf "\"uploader\":{\"name\":\"other\",\"digest\":\"%s\"}," "$digest"' \
		'case "${CLI_DIGEST:-$digest}" in' \
		'  missing) printf "\"digest\":null}," ;;' \
		'  *) printf "\"digest\":\"%s\"}," "${CLI_DIGEST:-$digest}" ;;' \
		'esac' \
		'if [ -n "${GNU_ARCHIVE:-}" ]; then' \
		'  checksum="$(shasum -a 256 "$GNU_ARCHIVE")"' \
		'  printf "{\"digest\":\"sha256:%s\",\"name\":\"xberg-cli-x86_64-unknown-linux-gnu.tar.gz\"}," "${checksum%% *}"' \
		'fi' \
		'printf "{\"name\":\"unrelated.tar.gz\",\"digest\":\"%s\"}],\"name\":\"release\"}\n" "$digest"'
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'[[ "$1 $3 $4 $5 $6" == "-q --proto =https --proto-redir =https" ]] || exit 94' \
		'mode="$2"; shift 6; authenticated=false' \
		'if [ "$1" = --header ]; then [ "$2" = @- ] || exit 94; shift 2; authenticated=true; fi' \
		'case "$1" in' \
		'  https://api.github.com/repos/xberg-io/xberg/releases/latest | https://api.github.com/repos/xberg-io/xberg/releases/tags/v1.4.0)' \
		'    [ "$mode" = -fsS ] && [ "$2" = -o ] || exit 94' \
		'    if [ "$authenticated" = true ]; then' \
		'      IFS= read -r header && [ "$header" = "Authorization: Bearer $GITHUB_TOKEN" ] || exit 94' \
		'      printf "%s\n" "$1" >>"$CLI_AUTH_TRACE"' \
		'    fi' \
		'    if [ "${CLI_METADATA_MODE:-}" = unavailable ]; then exit 22; fi' \
		'    if [ "${NO_RELEASE:-false}" = true ]; then printf "{}\n" >"$3"; exit; fi' \
		'    exec release-metadata >"$3" ;;' \
		'esac' \
		'[ "$authenticated" = false ] && [ "$mode" = -fsSL ] || exit 94' \
		'archive="$CLI_ARCHIVE"; case "$*" in *-linux-gnu.tar.gz*) if [ -n "${GNU_STATUS:-}" ]; then printf "%s" "$GNU_HTTP"; exit "$GNU_STATUS"; fi; archive="${GNU_ARCHIVE:-$archive}" ;; esac' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$archive" "$2"; exit; fi; shift; done' \
		'exit 91'
}

make_cli_archive() {
	local stage="source/xberg-cli-$CLI_TARGET"
	mkdir -p "$stage/lib/libheif"
	printf '#!/usr/bin/env bash\nprintf "xberg 1.4.0\\n"\n' >"$stage/xberg"
	printf 'native executable bytes\n' >"$stage/xberg.bin"
	printf 'nested runtime bytes\n' >"$stage/lib/libheif/libheif.so"
	printf 'flat runtime bytes\n' >"$stage/libsupport.dylib"
	tar -czf "$CLI_ARCHIVE" -C source "xberg-cli-$CLI_TARGET"
}

@test "CLI installer should install the selected Linux musl release with its launcher and nested runtime libraries" {
	make_cli_archive
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	cmp "source/xberg-cli-$CLI_TARGET/xberg" "$XBERG_INSTALL/xberg"
	xberg_assert_file "$XBERG_INSTALL/xberg.bin" 'native executable bytes'
	xberg_assert_file "$XBERG_INSTALL/lib/libheif/libheif.so" 'nested runtime bytes'
	xberg_assert_file "$XBERG_INSTALL/libsupport.dylib" 'flat runtime bytes'
	[ -x "$XBERG_INSTALL/xberg" ]
	[ -x "$XBERG_INSTALL/xberg.bin" ]
	[ "$(find "$XBERG_INSTALL" -type f | wc -l | tr -d '[:space:]')" -eq 4 ]
	xberg_assert_output_contains 'Verified: xberg 1.4.0'
	grep -Fq 'releases/download/v1.4.0/xberg-cli-aarch64-unknown-linux-musl.tar.gz' "$XBERG_TRACE"
}

@test "Packaged musl CLI launcher should fill only unset allocator settings" {
	local stage="$XBERG_WORK/wrapped cli" format
	mkdir -p "$stage/lib"
	format="$(
		python3 - "$REPOSITORY_ROOT/docker/Dockerfile.musl-build" <<'PY'
import shlex
import sys
from pathlib import Path

lines = Path(sys.argv[1]).read_text().splitlines()
commands = [line.strip().removesuffix("\\") for line in lines if line.lstrip().startswith("printf '#!/bin/sh")]
assert len(commands) == 1, "expected exactly one packaged launcher template"
tokens = shlex.split(commands[0])
assert len(tokens) == 2 and tokens[0] == "printf", "expected the launcher printf format"
print(tokens[1], end="")
PY
	)"
	printf "$format" ld-musl-test.so.1 >"$stage/xberg"
	cat >"$stage/lib/ld-musl-test.so.1" <<'SH'
#!/bin/sh
test "$1" = --library-path && test "$2" = "$(dirname "$0")" || exit 96
test "$3" = "$(dirname "$2")/xberg.bin" || exit 97
test "$4" = "preserve this arg" || exit 98
printf 'purge=[%s] thp=[%s] reset=[%s]\n' "${MIMALLOC_PURGE_DELAY-unset}" "${MIMALLOC_ALLOW_THP-unset}" "${MIMALLOC_RESET_DELAY-unset}"
SH
	chmod +x "$stage/xberg" "$stage/lib/ld-musl-test.so.1"

	unset MIMALLOC_PURGE_DELAY MIMALLOC_ALLOW_THP MIMALLOC_RESET_DELAY
	run "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[0] thp=[0]'

	run env MIMALLOC_PURGE_DELAY=50 MIMALLOC_ALLOW_THP=1 "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[50] thp=[1]'

	run env MIMALLOC_PURGE_DELAY= "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[] thp=[0]'

	run env MIMALLOC_RESET_DELAY=75 "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[unset] thp=[0] reset=[75]'

	run env MIMALLOC_RESET_DELAY= "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[unset] thp=[0] reset=[]'

	run env MIMALLOC_PURGE_DELAY=40 MIMALLOC_RESET_DELAY=75 "$stage/xberg" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[40] thp=[0] reset=[75]'

	run "$stage/lib/ld-musl-test.so.1" --library-path "$stage/lib" "$stage/xberg.bin" "preserve this arg"
	xberg_assert_status 0
	xberg_assert_output_contains 'purge=[unset] thp=[unset]'
}

@test "CLI publish smoke should verify and install an unpublished archive offline" {
	export CLI_ARCHIVE="$XBERG_WORK/xberg-cli-$CLI_TARGET.tar.gz"
	make_cli_archive
	local stage="source/xberg-cli-$CLI_TARGET"
	cat >"$stage/xberg" <<'SH'
#!/usr/bin/env bash
file="${2:-}"
case "$1:${file##*/}" in
--version:)
  printf 'xberg 1.4.0\n'
  if [ "${MIMALLOC_VERBOSE:-0}" = 1 ] && [ "${SMOKE_STUB_MIMALLOC:-0}" = 1 ]; then
    printf "mimalloc: process init\noption 'verbose': 1 \n" >&2
  fi
  ;;
extract:document.txt) printf 'Xberg installed layout smoke test\n' ;;
extract:musl-python-smoke.pdf) printf 'XBERG MUSL SMOKE 490\n' ;;
extract:merged_cells.docx) printf 'Blower\n' ;;
*) exit 95 ;;
esac
SH
	tar -czf "$CLI_ARCHIVE" -C source "xberg-cli-$CLI_TARGET"
	local smoke="$REPOSITORY_ROOT/scripts/ci/smoke-test-cli-installer.sh"
	run bash "$smoke" "$CLI_ARCHIVE" 1.4.0 musl
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified: xberg 1.4.0'

	run env SMOKE_REQUIRE_MIMALLOC=1 bash "$smoke" "$CLI_ARCHIVE" 1.4.0 musl
	xberg_assert_status 1
	xberg_assert_output_contains "Packaged CLI did not report mimalloc's verbose option"

	run env SMOKE_REQUIRE_MIMALLOC=1 SMOKE_STUB_MIMALLOC=1 bash "$smoke" "$CLI_ARCHIVE" 1.4.0 musl
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified: packaged CLI reports mimalloc'
	xberg_assert_output_contains 'Xberg installed layout smoke test'
	xberg_assert_trace_empty
}

@test "CLI installer should resolve the latest release and normalize the macOS amd64 target" {
	unset XBERG_VERSION
	export CLI_SYSTEM=Darwin CLI_ARCH=amd64 CLI_TARGET=x86_64-apple-darwin XBERG_LINUX_LIBC=ignored
	make_cli_archive
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	grep -Fq 'api.github.com/repos/xberg-io/xberg/releases/latest' "$XBERG_TRACE"
	grep -Fq 'releases/download/v1.4.0/xberg-cli-x86_64-apple-darwin.tar.gz' "$XBERG_TRACE"
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
	cmp "source/xberg-cli-$CLI_TARGET/xberg" "$XBERG_INSTALL/xberg"
	xberg_assert_file "$XBERG_INSTALL/libsupport.dylib" 'flat runtime bytes'
	xberg_assert_file "$XBERG_INSTALL/lib/libheif/libheif.so" 'nested runtime bytes'
}

@test "CLI installer should send optional credentials only to the GitHub API without exposing them" {
	make_cli_archive
	export GITHUB_TOKEN=ghp_test_placeholder_3193
	run bash -x "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	xberg_assert_file "$CLI_AUTH_TRACE" 'https://api.github.com/repos/xberg-io/xberg/releases/tags/v1.4.0'
	[[ "$output" != *"$GITHUB_TOKEN"* ]]
	! grep -Fq "$GITHUB_TOKEN" "$XBERG_TRACE"
	grep -Fq -- '-fsSL --proto =https --proto-redir =https' "$XBERG_TRACE"
}

@test "CLI installer should require a SHA-256 utility before reaching the network" {
	run env PATH="$(xberg_isolated_path)" "$BASH" "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "need 'sha256sum' or 'shasum'"
	xberg_assert_trace_empty
	[ ! -e "$XBERG_INSTALL" ]
}

@test "CLI installer should verify the archive with shasum when sha256sum is missing" {
	make_cli_archive
	ln -s "$(command -v shasum)" "$XBERG_STUB_BIN/shasum"
	run env PATH="$(xberg_isolated_path)" "$BASH" "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified: xberg 1.4.0'
	[ -x "$XBERG_INSTALL/xberg" ]
}

@test "CLI installer should reject unsupported operating systems before network access" {
	export CLI_SYSTEM=FreeBSD
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'unsupported OS: FreeBSD'
	xberg_assert_trace_empty
	[ ! -e "$XBERG_INSTALL" ]
}

@test "CLI installer should reject unsupported architectures before network access" {
	export CLI_ARCH=riscv64
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'unsupported architecture: riscv64'
	xberg_assert_trace_empty
	[ ! -e "$XBERG_INSTALL" ]
}

@test "CLI installer should reject a successful release query containing no version tags" {
	unset XBERG_VERSION
	export NO_RELEASE=true
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to fetch latest release tag from GitHub'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
	[ ! -e "$XBERG_INSTALL" ]
}

@test "CLI installer should preserve failed downloads without installing a launcher" {
	export CURL_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 7
	[ ! -e "$XBERG_INSTALL/xberg" ]
	[[ "$output" != *'Installed xberg to'* ]]
}

@test "CLI installer should reject a mismatched SHA-256 before extracting or trying musl" {
	export CLI_ARCH=amd64 CLI_TARGET=x86_64-unknown-linux-gnu
	xberg_stub ldd 'printf "GNU libc 2.39\\n"'
	make_cli_archive
	local checksum
	checksum="$(shasum -a 256 "$CLI_ARCHIVE")"
	export CLI_DIGEST="sha256:${checksum%% *}"
	printf 'corruption\n' >>"$CLI_ARCHIVE"
	xberg_stub tar 'printf "unexpected extraction\\n"; exit 92'
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'SHA-256 mismatch'
	[[ "$output" != *'unexpected extraction'* ]]
	[[ "$(cat "$XBERG_TRACE")" != *x86_64-unknown-linux-musl.tar.gz* ]]
	[ ! -e "$XBERG_INSTALL/xberg" ]
}

@test "CLI installer should reject a missing or invalid digest even if another asset has the archive hash" {
	make_cli_archive
	xberg_stub tar 'printf "unexpected extraction\\n"; exit 92'
	for CLI_DIGEST in missing sha256:bad sha1:0000000000000000000000000000000000000000000000000000000000000000; do
		export CLI_DIGEST
		run bash "$REPOSITORY_ROOT/scripts/install.sh"
		xberg_assert_status 1
		xberg_assert_output_contains 'no valid SHA-256 digest'
		[[ "$output" != *'unexpected extraction'* ]]
		[ ! -e "$XBERG_INSTALL/xberg" ]
	done
}

@test "CLI installer should reject ambiguous metadata and metadata without the selected asset" {
	make_cli_archive
	xberg_stub tar 'printf "unexpected extraction\\n"; exit 92'
	for CLI_METADATA_MODE in duplicate-digest duplicate-name duplicate-assets no-asset; do
		export CLI_METADATA_MODE
		run bash "$REPOSITORY_ROOT/scripts/install.sh"
		xberg_assert_status 1
		xberg_assert_output_contains 'no valid SHA-256 digest'
		[[ "$output" != *'unexpected extraction'* ]]
		[ ! -e "$XBERG_INSTALL/xberg" ]
	done
}

@test "CLI installer should reject an unavailable tag metadata response before extraction" {
	export CLI_METADATA_MODE=unavailable
	make_cli_archive
	xberg_stub tar 'printf "unexpected extraction\\n"; exit 92'
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to fetch SHA-256 metadata for v1.4.0'
	[[ "$output" != *'unexpected extraction'* ]]
	[ ! -e "$XBERG_INSTALL/xberg" ]
}

@test "CLI installer should reject a valid archive that contains no launcher" {
	mkdir -p source
	printf 'wrong artifact\n' >source/README
	tar -czf "$CLI_ARCHIVE" -C source README
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'binary not found in archive'
	[ ! -e "$XBERG_INSTALL/xberg" ]
}

@test "CLI installer should propagate installation copy failures without claiming an installed binary" {
	make_cli_archive
	export REAL_CP
	REAL_CP="$(command -v cp)"
	xberg_stub cp 'case "$2" in */installed/xberg) exit 9 ;; esac' 'exec "$REAL_CP" "$@"'
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 9
	[ ! -e "$XBERG_INSTALL/xberg" ]
	[[ "$output" != *'Installed xberg to'* ]]
}

@test "CLI installer should fall back to musl when the GNU asset returns HTTP 404" {
	export CLI_ARCH=amd64 CLI_TARGET=x86_64-unknown-linux-musl GNU_STATUS=22 GNU_HTTP=404
	xberg_stub ldd 'printf "GNU libc 2.36\n"'
	make_cli_archive
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified: xberg 1.4.0'
	grep -Fq 'releases/download/v1.4.0/xberg-cli-x86_64-unknown-linux-musl.tar.gz' "$XBERG_TRACE"
}

@test "CLI installer should prefer compatible GNU and preserve its flat runtime siblings" {
	export CLI_ARCH=amd64 CLI_TARGET=x86_64-unknown-linux-gnu
	xberg_stub ldd 'printf "GNU libc 2.39\n"'
	make_cli_archive
	local stage="source/xberg-cli-$CLI_TARGET"
	printf 'flat runtime bytes\n' >"$stage/libheif.so.1"
	printf '#!/usr/bin/env bash\n[ -f "${0%%/*}/libheif.so.1" ] || exit 1\nprintf "xberg 1.4.0\\n"\n' >"$stage/xberg"
	tar -czf "$CLI_ARCHIVE" -C source "xberg-cli-$CLI_TARGET"
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	xberg_assert_file "$XBERG_INSTALL/libheif.so.1" 'flat runtime bytes'
	xberg_assert_output_contains 'Verified: xberg 1.4.0'
	grep -Fq 'releases/download/v1.4.0/xberg-cli-x86_64-unknown-linux-gnu.tar.gz' "$XBERG_TRACE"
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
}

@test "CLI installer should reject incompatible Linux archives before replacing an existing CLI" {
	make_cli_archive
	local stage="source/xberg-cli-$CLI_TARGET"
	printf '#!/usr/bin/env bash\necho loader-error >&2\nexit 1\n' >"$stage/xberg"
	tar -czf "$CLI_ARCHIVE" -C source "xberg-cli-$CLI_TARGET"
	mkdir -p "$XBERG_INSTALL"
	printf 'existing executable\n' >"$XBERG_INSTALL/xberg"
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'loader-error'
	xberg_assert_file "$XBERG_INSTALL/xberg" 'existing executable'
}

@test "CLI installer should fall back when the GNU executable cannot start" {
	export CLI_ARCH=amd64 CLI_TARGET=x86_64-unknown-linux-musl
	xberg_stub ldd 'printf "GNU libc 2.36\n"'
	make_cli_archive
	local stage="source/xberg-cli-x86_64-unknown-linux-gnu"
	mkdir -p "$stage"
	printf '#!/usr/bin/env bash\necho GLIBC_2.38-not-found >&2\nexit 1\n' >"$stage/xberg"
	export GNU_ARCHIVE="$XBERG_WORK/gnu.tar.gz"
	tar -czf "$GNU_ARCHIVE" -C source "xberg-cli-x86_64-unknown-linux-gnu"
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'GLIBC_2.38-not-found'
	xberg_assert_output_contains 'Verified: xberg 1.4.0'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 3 ]
}

@test "CLI installer should not fall back on server or transport errors" {
	export CLI_ARCH=amd64 CLI_TARGET=x86_64-unknown-linux-musl
	xberg_stub ldd 'printf "GNU libc 2.36\n"'
	make_cli_archive
	for failure in '22 503' '28 000'; do
		read -r GNU_STATUS GNU_HTTP <<<"$failure"
		export GNU_STATUS GNU_HTTP
		: >"$XBERG_TRACE"
		run bash "$REPOSITORY_ROOT/scripts/install.sh"
		xberg_assert_status "$GNU_STATUS"
		[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
		[ ! -e "$XBERG_INSTALL/xberg" ]
	done
}

@test "CLI installer should not fall back for an explicit GNU selection" {
	export CLI_ARCH=amd64 XBERG_LINUX_LIBC=gnu GNU_STATUS=22 GNU_HTTP=404
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 22
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
	[ ! -e "$XBERG_INSTALL/xberg" ]
}
