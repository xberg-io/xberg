setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_shadow_system_path_without brew nasm
	PATH="$(xberg_isolated_path)"
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/build-macos-heif-deps.sh"
	export XBERG_HEIF_PREFIX="$XBERG_WORK/heif" MACOSX_DEPLOYMENT_TARGET=11.0
	mkdir -p "$XBERG_HEIF_PREFIX/lib/pkgconfig"
	touch "$XBERG_HEIF_PREFIX/lib/libheif.dylib"
	xberg_stub_exit curl 42
	xberg_stub_exit sleep 0
	xberg_stub sysctl 'printf "4\n"'
	xberg_stub uname 'printf "x86_64\n"'
	xberg_stub brew 'printf "brew %s\n" "$*" >>"$XBERG_TRACE"' '[ "$*" = "install nasm" ]'
	xberg_stub pkg-config 'sed -n "s/^Version: //p" "$PKG_CONFIG_LIBDIR/${!#}.pc"'
}

@test "macos_heif should_rebuild_a_prefix_without_a_version_receipt" {
	run "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "libde265 1.1.3"
	xberg_assert_trace 'brew install nasm'
}

@test "macos_heif should_reject_an_old_receipt_before_reusing_a_prefix" {
	printf '%s\n' 'libheif=1.23.0 libde265=1.0.16 libaom=3.12.1 target=11.0' \
		>"$XBERG_HEIF_PREFIX/.xberg-heif-versions"
	run "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "libde265 1.1.3"
	xberg_assert_trace 'brew install nasm'
}

@test "macos_heif should_reject_a_current_receipt_with_stale_pkg_config_versions" {
	printf 'libheif=1.23.7 libde265=1.1.3 libaom=3.15.2 target=11.0 arch=%s\n' "$(uname -m)" \
		>"$XBERG_HEIF_PREFIX/.xberg-heif-versions"
	xberg_stub pkg-config 'printf "%s\n" 1.23.0'
	run "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "libheif version 1.23.0 does not match 1.23.7"
	xberg_assert_output_contains "libde265 1.1.3"
	xberg_assert_trace 'brew install nasm'
}

@test "macos_heif should_reject_corrupt_download_before_extracting_it" {
	awk '/^build_dep\(\)/{capture=1} /^deps_of\(\)/{capture=0} capture' "$SCRIPT" >"$XBERG_WORK/build-dep.sh"
	[ "$(grep -c '^build_dep()' "$XBERG_WORK/build-dep.sh")" -eq 1 ]
	xberg_stub curl 'while [ "$1" != "-o" ]; do shift; done; printf corrupt >"$2"'
	xberg_stub tar 'touch "$XBERG_WORK/tar-was-called"'
	run "$BASH" -euo pipefail -c 'source "$1"; source "$2"; WORK="$3"; build_dep fake 1 https://example.invalid/archive 0000000000000000000000000000000000000000000000000000000000000000' \
		_ "$REPO_ROOT/scripts/lib/retry.sh" "$XBERG_WORK/build-dep.sh" "$XBERG_WORK"
	xberg_assert_status 1
	xberg_assert_output_contains "FAILED"
	[ ! -e "$XBERG_WORK/tar-was-called" ]
}

@test "macos_heif should_reuse_only_a_complete_matching_version_and_target_prefix" {
	printf 'libheif=1.23.7 libde265=1.1.3 libaom=3.15.2 target=11.0 arch=%s\n' "$(uname -m)" \
		>"$XBERG_HEIF_PREFIX/.xberg-heif-versions"
	local spec name version
	for spec in libheif:1.23.7 libde265:1.1.3 aom:3.15.2; do
		name="${spec%%:*}"
		version="${spec#*:}"
		printf 'Name: %s\nDescription: version fixture\nVersion: %s\nLibs:\n' "$name" "$version" \
			>"$XBERG_HEIF_PREFIX/lib/pkgconfig/$name.pc"
	done
	touch "$XBERG_HEIF_PREFIX/lib/libde265.dylib" "$XBERG_HEIF_PREFIX/lib/libaom.dylib"
	xberg_stub otool 'if [ "$1" = -l ]; then printf "LC_BUILD_VERSION\nminos 11.0\n"; else printf "fixture:\nidentity:\n"; fi'
	run "$BASH" "$SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains "Verified libheif stack already present"
	xberg_assert_trace_empty
}

@test "macos_heif should_extract_a_verified_download_before_running_cmake" {
	awk '/^build_dep\(\)/{capture=1} /^deps_of\(\)/{capture=0} capture' "$SCRIPT" >"$XBERG_WORK/build-dep.sh"
	[ "$(grep -c '^build_dep()' "$XBERG_WORK/build-dep.sh")" -eq 1 ]
	xberg_stub curl 'while [ "$1" != "-o" ]; do shift; done; printf abc >"$2"'
	xberg_stub tar 'touch "$XBERG_WORK/tar-was-called"'
	xberg_stub cmake 'printf "cmake\n" >>"$XBERG_WORK/cmake-calls"'
	run "$BASH" -euo pipefail -c 'source "$1"; source "$2"; WORK="$3"; JOBS=1; CMAKE_COMMON=(-Dfixture=true); build_dep fake 1 https://example.invalid/archive ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad' \
		_ "$REPO_ROOT/scripts/lib/retry.sh" "$XBERG_WORK/build-dep.sh" "$XBERG_WORK"
	xberg_assert_status 0
	[ -e "$XBERG_WORK/tar-was-called" ]
	[ "$(wc -l <"$XBERG_WORK/cmake-calls" | tr -d ' ')" -eq 3 ]
}

@test "macos_heif should_reject_an_invalid_parallel_build_limit_before_downloading" {
	run env CMAKE_BUILD_PARALLEL_LEVEL=zero "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "CMAKE_BUILD_PARALLEL_LEVEL must be a positive integer"
}

@test "macos_heif should_pass_an_explicit_one_job_limit_to_the_actual_build_function" {
	awk '/^JOBS=/{capture=1} /^# Absolute install names/{capture=0} capture' "$SCRIPT" >"$XBERG_WORK/jobs.sh"
	[ "$(grep -c '^JOBS=' "$XBERG_WORK/jobs.sh")" -eq 1 ]
	awk '/^build_dep\(\)/{capture=1} /^deps_of\(\)/{capture=0} capture' "$SCRIPT" >"$XBERG_WORK/build-dep.sh"
	[ "$(grep -c '^build_dep()' "$XBERG_WORK/build-dep.sh")" -eq 1 ]
	xberg_stub curl 'while [ "$1" != "-o" ]; do shift; done; printf abc >"$2"'
	xberg_stub_exit tar 0
	xberg_stub cmake 'printf "%s\n" "$*" >>"$XBERG_WORK/cmake-arguments"'
	run env CMAKE_BUILD_PARALLEL_LEVEL=1 "$BASH" -euo pipefail -c 'source "$1"; source "$2"; source "$3"; WORK="$4"; CMAKE_COMMON=(-Dfixture=true); build_dep fake 1 https://example.invalid/archive ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad' \
		_ "$REPO_ROOT/scripts/lib/retry.sh" "$XBERG_WORK/jobs.sh" "$XBERG_WORK/build-dep.sh" "$XBERG_WORK"
	xberg_assert_status 0
	[ "$(grep -c -- '--build .* -j 1$' "$XBERG_WORK/cmake-arguments")" -eq 1 ]
}
