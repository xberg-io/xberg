#!/usr/bin/env bats
#
# Contract tests for scripts/pdfium/pdfium.sh and the pdfium:* mise tasks that wrap it. curl, gh and
# cargo are stubs, so no archive is downloaded and no build runs.

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	PDFIUM_SH="$REPOSITORY_ROOT/scripts/pdfium/pdfium.sh"
	export TMPDIR="$BATS_TEST_TMPDIR/tmp" PDFIUM_CACHE_DIR="$BATS_TEST_TMPDIR/cache"
	export PDFIUM_VERSION=1 PDFIUM_CHECKSUMS_FILE="$BATS_TEST_TMPDIR/checksums.sha256"
	export PDFIUM_FIXTURE_ARCHIVE="$BATS_TEST_TMPDIR/pdfium.tgz"
	mkdir -p "$TMPDIR" "$BATS_TEST_TMPDIR/source/lib"
	unset PDFIUM_DYNAMIC_LIB_PATH
	xberg_stub curl \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$PDFIUM_FIXTURE_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
}

# Pins the archive under every asset name the manifest may carry.
pin_archive() {
	local sum
	tar -czf "$PDFIUM_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/source" lib
	sum="$(shasum -a 256 "$PDFIUM_FIXTURE_ARCHIVE")"
	: >"$PDFIUM_CHECKSUMS_FILE"
	for platform in linux-x64 mac-x64 mac-arm64; do
		printf '%s  1-pdfium-%s.tgz\n' "${sum%% *}" "$platform" >>"$PDFIUM_CHECKSUMS_FILE"
	done
}

good_archive() {
	printf 'library bytes\n' >"$BATS_TEST_TMPDIR/source/lib/libpdfium.so"
	cp "$BATS_TEST_TMPDIR/source/lib/libpdfium.so" "$BATS_TEST_TMPDIR/source/lib/libpdfium.dylib"
	pin_archive
}

# A gzip stream cut off after the library was written: tar exits non-zero with the file present.
truncated_archive() {
	head -c 2000000 /dev/urandom >"$BATS_TEST_TMPDIR/source/lib/libpdfium.so"
	pin_archive
	head -c 1500000 "$PDFIUM_FIXTURE_ARCHIVE" >"$PDFIUM_FIXTURE_ARCHIVE.cut"
	mv "$PDFIUM_FIXTURE_ARCHIVE.cut" "$PDFIUM_FIXTURE_ARCHIVE"
	local sum
	sum="$(shasum -a 256 "$PDFIUM_FIXTURE_ARCHIVE")"
	sed -i.bak "s/^[0-9a-f]*/${sum%% *}/" "$PDFIUM_CHECKSUMS_FILE"
}

@test "pdfium env should install nothing when extraction fails part way" {
	truncated_archive
	run bash "$PDFIUM_SH" env --platform linux-x64
	[ "$status" -ne 0 ]
	[ ! -e "$PDFIUM_CACHE_DIR/1/linux-x64" ]
}

@test "pdfium env should print the provisioned directory for a good archive" {
	good_archive
	run bash "$PDFIUM_SH" env --platform linux-x64
	[ "$status" -eq 0 ]
	[[ "$output" == *"export PDFIUM_DYNAMIC_LIB_PATH=$PDFIUM_CACHE_DIR/1/linux-x64/lib"* ]]
	[[ "$output" == *"export XBERG_REQUIRE_PDFIUM=1"* ]]
}

@test "pdfium bump should clean its scratch files and exit zero after a dry run" {
	good_archive
	local sum
	sum="$(shasum -a 256 "$PDFIUM_FIXTURE_ARCHIVE")"
	xberg_stub gh \
		'for platform in linux-x64 linux-arm64 linux-musl-x64 linux-musl-arm64 mac-x64 mac-arm64 win-x64 win-arm64; do' \
		"  echo \"sha256:${sum%% *} pdfium-\$platform.tgz\"" \
		'done'
	unset PDFIUM_CHECKSUMS_FILE PDFIUM_VERSION
	run bash "$PDFIUM_SH" bump 1 --allow-binding-mismatch
	[ "$status" -eq 0 ] || { echo "$output"; return 1; }
	[[ "$output" == *"Dry run"* ]]
	[ -z "$(ls -A "$TMPDIR")" ] || { ls -A "$TMPDIR"; return 1; }
}

@test "pdfium:provision should repeat the selected platform and version in its env hint" {
	good_archive
	export usage_platform=mac-x64 usage_version=1
	run bash "$REPOSITORY_ROOT/mise-tasks/pdfium/provision.sh"
	[ "$status" -eq 0 ]
	[[ "$output" == *"pdfium:env --version 1 --platform mac-x64"* ]]
}

@test "pdfium:test should fail without running cargo when the pdfium environment cannot be built" {
	good_archive
	export PDFIUM_DYNAMIC_LIB_PATH=/already/set
	sed -i.bak 's/^[0-9a-f]\{64\}/0000000000000000000000000000000000000000000000000000000000000000/' "$PDFIUM_CHECKSUMS_FILE"
	xberg_stub cargo 'echo ran >>"$XBERG_TRACE"'
	run bash "$REPOSITORY_ROOT/mise-tasks/pdfium/test.sh"
	[ "$status" -ne 0 ]
	[ ! -e "$XBERG_TRACE" ]
}
