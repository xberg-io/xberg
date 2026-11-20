#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	unset ORT_CHECKSUMS_FILE
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/actions/setup-onnx-runtime/macos.sh"
	export RUNNER_TEMP="$XBERG_WORK/runner" GITHUB_WORKSPACE="$XBERG_WORK/workspace"
	export RUSTFLAGS='' DYLD_LIBRARY_PATH='/ambient/dyld'
	export DYLD_FALLBACK_LIBRARY_PATH='/ambient/fallback' LIBRARY_PATH='/ambient/lib'
	ORT_ROOT="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-arm64-1.24.2"
	DEST="$GITHUB_WORKSPACE/native"
	mkdir -p "$ORT_ROOT/lib" "$GITHUB_WORKSPACE"
	printf 'cached-ort-fixture\n' >"$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib"
	xberg_stub curl 'printf "curl\n" >>"$XBERG_TRACE"; exit 99'
	xberg_stub tar 'printf "tar\n" >>"$XBERG_TRACE"; exit 99'
}

run_setup_script() {
	# ~keep SIP strips DYLD_* when launching /bin/bash; install fixture values inside the child shell.
	run bash -c '
		export DYLD_LIBRARY_PATH="$1" DYLD_FALLBACK_LIBRARY_PATH="$2"
		shift 2
		source "$@"
	' bash /ambient/dyld /ambient/fallback "$SCRIPT" "$@"
}

assert_system_environment() {
	local flags="$1"
	cat >"$XBERG_WORK/expected-env" <<EXPECTED
ORT_LIB_LOCATION=$ORT_ROOT/lib
ORT_PREFER_DYNAMIC_LINK=1
ORT_SKIP_DOWNLOAD=1
ORT_STRATEGY=system
ORT_DYLIB_PATH=$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib
DYLD_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/dyld
DYLD_FALLBACK_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/fallback
LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/lib
RUSTFLAGS=$flags
EXPECTED
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 9 ]
	cmp "$XBERG_WORK/expected-env" "$GITHUB_ENV"
	cmp "$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib" "$DEST/libonnxruntime.1.24.2.dylib"
	[ ! -s "$XBERG_TRACE" ]
}

assert_bundled_environment() {
	cat >"$XBERG_WORK/expected-env" <<EXPECTED
DYLD_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/dyld
DYLD_FALLBACK_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/fallback
LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/lib
EXPECTED
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 3 ]
	cmp "$XBERG_WORK/expected-env" "$GITHUB_ENV"
	cmp "$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib" "$DEST/libonnxruntime.1.24.2.dylib"
	[ ! -s "$XBERG_TRACE" ]
}

@test "setup_onnx_macos should_keep_default_rpath_and_all_system_environment_rows" {
	run_setup_script 1.24.2 native arm64 system
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_accept_explicit_true_rpath" {
	run_setup_script 1.24.2 native arm64 system true
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_preserve_ambient_flags_with_default_rpath" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run_setup_script 1.24.2 native arm64 system
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_omit_setup_rpath_when_the_package_owns_it" {
	run_setup_script 1.24.2 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_ambient_flags_and_ort_search_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run_setup_script 1.24.2 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_default_bundled_environment" {
	run_setup_script 1.24.2 native arm64 bundled
	xberg_assert_status 0
	assert_bundled_environment
}

@test "setup_onnx_macos should_preserve_bundled_environment_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings'
	run_setup_script 1.24.2 native arm64 bundled false
	xberg_assert_status 0
	assert_bundled_environment
}

assert_invalid_rpath() {
	run_setup_script 1.24.2 native arm64 system "$1"
	xberg_assert_status 2
	xberg_assert_output_contains 'add-macos-rpath must be true or false'
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
	[ ! -s "$XBERG_TRACE" ]
}

@test "setup_onnx_macos should_reject_invalid_rpath_before_staging" {
	assert_invalid_rpath unexpected
}

@test "setup_onnx_macos should_reject_explicit_empty_rpath_before_staging" {
	assert_invalid_rpath ''
}

@test "setup_onnx_macos should_reject_case_variant_rpath_before_staging" {
	assert_invalid_rpath TRUE
}

prepare_fresh_archive() {
	local name="$1" version="$2" checksum
	rm -rf "$RUNNER_TEMP/onnxruntime"
	export ORT_FIXTURE="$XBERG_WORK/$name.tgz"
	export ORT_CHECKSUMS_FILE="$XBERG_WORK/checksums.sha256"
	mkdir -p "$XBERG_WORK/fixture/$name/lib"
	printf 'verified dylib\n' >"$XBERG_WORK/fixture/$name/lib/libonnxruntime.$version.dylib"
	command -p tar -czf "$ORT_FIXTURE" -C "$XBERG_WORK/fixture" "$name"
	checksum="$(shasum -a 256 "$ORT_FIXTURE")"
	printf '%s  %s.tgz\n' "${checksum%% *}" "$name" >"$ORT_CHECKSUMS_FILE"
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE" "$2"; exit; fi; shift; done' \
		'exit 91'
	rm "$XBERG_STUB_BIN/tar"
}

@test "setup_onnx_macos verifies a fresh pinned arm64 archive before staging it" {
	prepare_fresh_archive onnxruntime-osx-arm64-1.24.2 1.24.2
	run_setup_script 1.24.2 native arm64 system
	xberg_assert_status 0
	xberg_assert_output_contains 'onnxruntime-osx-arm64-1.24.2.tgz: SHA-256 verified'
	xberg_assert_file "$DEST/libonnxruntime.1.24.2.dylib" 'verified dylib'
	grep -Fq 'download/v1.24.2/onnxruntime-osx-arm64-1.24.2.tgz' "$XBERG_TRACE"
}

@test "setup_onnx_macos verifies the existing Intel fallback release" {
	prepare_fresh_archive onnxruntime-osx-x86_64-1.23.2 1.23.2
	run_setup_script 1.24.2 native x64 bundled
	xberg_assert_status 0
	xberg_assert_output_contains 'onnxruntime-osx-x86_64-1.23.2.tgz: SHA-256 verified'
	xberg_assert_file "$DEST/libonnxruntime.1.23.2.dylib" 'verified dylib'
	grep -Fq 'download/v1.23.2/onnxruntime-osx-x86_64-1.23.2.tgz' "$XBERG_TRACE"
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 3 ]
}

@test "setup_onnx_macos rejects an unpinned fresh version before curl" {
	unset ORT_CHECKSUMS_FILE
	run_setup_script 9.9.9 native arm64 system
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for onnxruntime-osx-arm64-9.9.9.tgz'
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
	xberg_assert_trace_empty
}

@test "setup_onnx_macos rejects mismatched bytes before tar or publishing" {
	prepare_fresh_archive onnxruntime-osx-arm64-1.24.2 1.24.2
	printf 'tampered archive\n' >"$ORT_FIXTURE"
	xberg_stub_exit tar 93
	run_setup_script 1.24.2 native arm64 system
	xberg_assert_status 1
	xberg_assert_output_contains 'SHA-256 mismatch for onnxruntime-osx-arm64-1.24.2.tgz'
	[ ! -e "$RUNNER_TEMP/onnxruntime" ]
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
}

@test "setup_onnx_macos rebuilds the Intel release layout from the pinned NuGet package when GitHub fails" {
	rm -rf "$RUNNER_TEMP/onnxruntime" "$XBERG_STUB_BIN/tar"
	export ORT_FIXTURE_PACKAGE="$XBERG_WORK/package.nupkg" ORT_CHECKSUMS_FILE="$XBERG_WORK/checksums.sha256"
	python3 - "$ORT_FIXTURE_PACKAGE" <<'PYTHON'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], "w") as package:
    package.writestr("runtimes/osx-x64/native/libonnxruntime.dylib", "intel nuget dylib\n")
    package.writestr("runtimes/osx-arm64/native/libonnxruntime.dylib", "arm nuget dylib\n")
    package.writestr("build/native/include/onnxruntime_c_api.h", "header\n")
PYTHON
	checksum="$(shasum -a 256 "$ORT_FIXTURE_PACKAGE")"
	{
		printf '%s  onnxruntime-osx-x86_64-1.23.2.tgz\n' "$(printf 'a%.0s' {1..64})"
		printf '%s  microsoft.ml.onnxruntime.1.23.2.nupkg\n' "${checksum%% *}"
	} >"$ORT_CHECKSUMS_FILE"
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *github.com*) exit 22 ;; esac' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE_PACKAGE" "$2"; exit; fi; shift; done' \
		'exit 91'
	intel_root="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-x86_64-1.23.2"
	run_setup_script 1.24.2 native x64 system
	xberg_assert_status 0
	xberg_assert_output_contains 'GitHub release download of onnxruntime-osx-x86_64-1.23.2.tgz failed'
	xberg_assert_output_contains 'microsoft.ml.onnxruntime.1.23.2.nupkg: SHA-256 verified'
	xberg_assert_file "$intel_root/lib/libonnxruntime.1.23.2.dylib" 'intel nuget dylib'
	[ "$(readlink "$intel_root/lib/libonnxruntime.dylib")" = libonnxruntime.1.23.2.dylib ]
	xberg_assert_file "$DEST/libonnxruntime.1.23.2.dylib" 'intel nuget dylib'
	grep -Fq 'microsoft.ml.onnxruntime/1.23.2/microsoft.ml.onnxruntime.1.23.2.nupkg' "$XBERG_TRACE"
}
