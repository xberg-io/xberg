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
	ORT_ROOT="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-arm64-1.32.0"
	DEST="$GITHUB_WORKSPACE/native"
	mkdir -p "$ORT_ROOT/lib" "$GITHUB_WORKSPACE"
	printf 'cached-ort-fixture\n' >"$ORT_ROOT/lib/libonnxruntime.1.32.0.dylib"
	xberg_stub curl 'printf "curl\n" >>"$XBERG_TRACE"; exit 99'
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
ORT_DYLIB_PATH=$ORT_ROOT/lib/libonnxruntime.1.32.0.dylib
DYLD_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/dyld
DYLD_FALLBACK_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/fallback
LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/lib
RUSTFLAGS=$flags
EXPECTED
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 9 ]
	cmp "$XBERG_WORK/expected-env" "$GITHUB_ENV"
	cmp "$ORT_ROOT/lib/libonnxruntime.1.32.0.dylib" "$DEST/libonnxruntime.1.32.0.dylib"
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
	cmp "$ORT_ROOT/lib/libonnxruntime.1.32.0.dylib" "$DEST/libonnxruntime.1.32.0.dylib"
	[ ! -s "$XBERG_TRACE" ]
}

@test "setup_onnx_macos should_keep_default_rpath_and_all_system_environment_rows" {
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_accept_explicit_true_rpath" {
	run_setup_script 1.32.0 native arm64 system true
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_preserve_ambient_flags_with_default_rpath" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_omit_setup_rpath_when_the_package_owns_it" {
	run_setup_script 1.32.0 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_ambient_flags_and_ort_search_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run_setup_script 1.32.0 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_default_bundled_environment" {
	run_setup_script 1.32.0 native arm64 bundled
	xberg_assert_status 0
	assert_bundled_environment
}

@test "setup_onnx_macos should_preserve_bundled_environment_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings'
	run_setup_script 1.32.0 native arm64 bundled false
	xberg_assert_status 0
	assert_bundled_environment
}

assert_invalid_rpath() {
	run_setup_script 1.32.0 native arm64 system "$1"
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

# Writes a NuGet package and a PyPI wheel fixture for one macOS runtime, a manifest pinning both,
# and a curl that serves them. ORT_NUGET_MODE and ORT_PYPI_MODE pick ok, fail or tamper.
prepare_sources() {
	local rid="$1" version="$2" platform="$3" package_sum wheel_sum
	rm -rf "$RUNNER_TEMP/onnxruntime"
	export ORT_FIXTURE_PACKAGE="$XBERG_WORK/package.nupkg" ORT_FIXTURE_WHEEL="$XBERG_WORK/runtime.whl"
	export ORT_CHECKSUMS_FILE="$XBERG_WORK/checksums.sha256" ORT_NUGET_MODE=ok ORT_PYPI_MODE=ok
	ORT_WHEEL="onnxruntime-$version-cp312-cp312-macosx_$platform.whl"
	python3 - "$ORT_FIXTURE_PACKAGE" "$ORT_FIXTURE_WHEEL" "$rid" "$version" <<'PYTHON'
import sys
import zipfile

package, wheel, rid, version = sys.argv[1:]
with zipfile.ZipFile(package, "w") as archive:
    archive.writestr(f"runtimes/{rid}/native/libonnxruntime.dylib", "nuget dylib\n")
    archive.writestr("runtimes/osx-other/native/libonnxruntime.dylib", "wrong dylib\n")
    archive.writestr("build/native/include/onnxruntime_c_api.h", "header\n")
with zipfile.ZipFile(wheel, "w") as archive:
    archive.writestr(f"onnxruntime/capi/libonnxruntime.{version}.dylib", "pypi dylib\n")
    archive.writestr("onnxruntime/capi/onnxruntime_pybind11_state.so", "python extension\n")
PYTHON
	package_sum="$(shasum -a 256 "$ORT_FIXTURE_PACKAGE")"
	wheel_sum="$(shasum -a 256 "$ORT_FIXTURE_WHEEL")"
	{
		printf '%s  microsoft.ml.onnxruntime.%s.nupkg\n' "${package_sum%% *}" "$version"
		printf '%s  %s  %s  https://files.pythonhosted.org/packages/aa/bb/%s\n' "${wheel_sum%% *}" "$ORT_WHEEL" "$rid" "$ORT_WHEEL"
	} >"$ORT_CHECKSUMS_FILE"
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *--version*) exit 91 ;; *api.nuget.org*) mode="$ORT_NUGET_MODE" fixture="$ORT_FIXTURE_PACKAGE" ;; *files.pythonhosted.org*) mode="$ORT_PYPI_MODE" fixture="$ORT_FIXTURE_WHEEL" ;; *) exit 92 ;; esac' \
		'[ "$mode" != fail ] || exit 22' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then if [ "$mode" = tamper ]; then printf "tampered\n" >"$2"; else cp "$fixture" "$2"; fi; exit; fi; shift; done' \
		'exit 91'
}

downloads() {
	grep -v -e '--version' "$XBERG_TRACE"
}

assert_macos_layout() {
	local root="$1" version="$2" content="$3"
	xberg_assert_file "$root/lib/libonnxruntime.$version.dylib" "$content"
	[ "$(readlink "$root/lib/libonnxruntime.1.dylib")" = "libonnxruntime.$version.dylib" ]
	[ "$(readlink "$root/lib/libonnxruntime.dylib")" = "libonnxruntime.$version.dylib" ]
	xberg_assert_file "$DEST/libonnxruntime.$version.dylib" "$content"
	xberg_assert_file "$DEST/libonnxruntime.1.dylib" "$content"
}

@test "setup_onnx_macos stages the verified arm64 NuGet package" {
	prepare_sources osx-arm64 1.32.0 14_0_arm64
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 0
	xberg_assert_output_contains 'microsoft.ml.onnxruntime.1.32.0.nupkg: SHA-256 verified'
	assert_macos_layout "$ORT_ROOT" 1.32.0 'nuget dylib'
	xberg_assert_file "$ORT_ROOT/include/onnxruntime_c_api.h" 'header'
	[ "$(downloads | wc -l | tr -d ' ')" -eq 1 ]
	downloads | grep -Fq 'microsoft.ml.onnxruntime/1.32.0/microsoft.ml.onnxruntime.1.32.0.nupkg'
}

@test "setup_onnx_macos stages the Intel runtime from the last x86_64 release" {
	prepare_sources osx-x64 1.23.2 13_0_x86_64
	intel_root="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-x86_64-1.23.2"
	run_setup_script 1.32.0 native x64 bundled
	xberg_assert_status 0
	xberg_assert_output_contains 'microsoft.ml.onnxruntime.1.23.2.nupkg: SHA-256 verified'
	assert_macos_layout "$intel_root" 1.23.2 'nuget dylib'
	downloads | grep -Fq 'microsoft.ml.onnxruntime/1.23.2/microsoft.ml.onnxruntime.1.23.2.nupkg'
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 3 ]
}

@test "setup_onnx_macos falls back to the verified PyPI wheel when the NuGet download fails" {
	prepare_sources osx-arm64 1.32.0 14_0_arm64
	export ORT_NUGET_MODE=fail
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 0
	xberg_assert_output_contains 'NuGet download of microsoft.ml.onnxruntime.1.32.0.nupkg failed; using the PyPI wheel'
	xberg_assert_output_contains "$ORT_WHEEL: SHA-256 verified"
	assert_macos_layout "$ORT_ROOT" 1.32.0 'pypi dylib'
	[ ! -e "$ORT_ROOT/lib/onnxruntime_pybind11_state.so" ]
	[ "$(downloads | wc -l | tr -d ' ')" -eq 2 ]
	downloads | sed -n 2p | grep -Fq "https://files.pythonhosted.org/packages/aa/bb/$ORT_WHEEL"
}

@test "setup_onnx_macos falls back to the Intel PyPI wheel when the NuGet package fails verification" {
	prepare_sources osx-x64 1.23.2 13_0_x86_64
	export ORT_NUGET_MODE=tamper
	intel_root="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-x86_64-1.23.2"
	run_setup_script 1.32.0 native x64 system
	xberg_assert_status 0
	xberg_assert_output_contains 'SHA-256 mismatch for microsoft.ml.onnxruntime.1.23.2.nupkg'
	xberg_assert_output_contains "$ORT_WHEEL: SHA-256 verified"
	assert_macos_layout "$intel_root" 1.23.2 'pypi dylib'
}

@test "setup_onnx_macos rejects a tampered wheel before staging or publishing" {
	prepare_sources osx-arm64 1.32.0 14_0_arm64
	export ORT_NUGET_MODE=fail ORT_PYPI_MODE=tamper
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 1
	xberg_assert_output_contains "SHA-256 mismatch for $ORT_WHEEL"
	[ ! -e "$RUNNER_TEMP/onnxruntime" ]
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
}

@test "setup_onnx_macos rejects a version without NuGet or PyPI pins before curl" {
	rm -rf "$RUNNER_TEMP/onnxruntime"
	run_setup_script 9.9.9 native arm64 system
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for microsoft.ml.onnxruntime.9.9.9.nupkg'
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
	xberg_assert_trace_empty
}

@test "setup_onnx_macos rejects a pinned NuGet package without a pinned wheel before curl" {
	prepare_sources osx-arm64 1.32.0 14_0_arm64
	sed -i.bak '/\.whl/d' "$ORT_CHECKSUMS_FILE"
	run_setup_script 1.32.0 native arm64 system
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned PyPI wheel for ONNX Runtime 1.32.0 (osx-arm64)'
	xberg_assert_trace_empty
}
