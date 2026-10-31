#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	export XBERG_INSTALL="$XBERG_WORK/installed" XBERG_VERSION=1.4.0
	export CLI_SYSTEM=Linux CLI_ARCH=arm64 CLI_TARGET=aarch64-unknown-linux-musl
	export CLI_ARCHIVE="$XBERG_WORK/cli.tar.gz"
	unset CURL_STATUS NO_RELEASE
	xberg_stub uname 'case "$1" in -s) printf "%s\n" "$CLI_SYSTEM" ;; -m) printf "%s\n" "$CLI_ARCH" ;; *) exit 97 ;; esac'
	xberg_stub ldd 'printf "musl libc\n"'
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'case "$*" in *api.github.com*) if [ "${NO_RELEASE:-false}" = true ]; then printf "[]\n"; else printf "{\"tag_name\": \"v1.4.0\"}\n"; fi; exit 0 ;; esac' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$CLI_ARCHIVE" "$2"; exit; fi; shift; done' \
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

@test "CLI installer should resolve the latest release and normalize the macOS amd64 target" {
	unset XBERG_VERSION
	export CLI_SYSTEM=Darwin CLI_ARCH=amd64 CLI_TARGET=x86_64-apple-darwin
	make_cli_archive
	run bash "$REPOSITORY_ROOT/scripts/install.sh"
	xberg_assert_status 0
	grep -Fq 'api.github.com/repos/xberg-io/xberg/releases?per_page=20' "$XBERG_TRACE"
	grep -Fq 'releases/download/v1.4.0/xberg-cli-x86_64-apple-darwin.tar.gz' "$XBERG_TRACE"
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
	cmp "source/xberg-cli-$CLI_TARGET/xberg" "$XBERG_INSTALL/xberg"
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
