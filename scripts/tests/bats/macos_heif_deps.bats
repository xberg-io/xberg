setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_shadow_system_path_without brew nasm
	PATH="$(xberg_isolated_path)"
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/build-macos-heif-deps.sh"
	export XBERG_HEIF_PREFIX="$XBERG_WORK/heif" MACOSX_DEPLOYMENT_TARGET=15.0
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
	printf 'libheif=1.23.7 libde265=1.1.3 libaom=3.15.2 target=15.0 arch=%s\n' "$(uname -m)" \
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
	printf 'libheif=1.23.7 libde265=1.1.3 libaom=3.15.2 target=15.0 arch=%s\n' "$(uname -m)" \
		>"$XBERG_HEIF_PREFIX/.xberg-heif-versions"
	local spec name version
	for spec in libheif:1.23.7 libde265:1.1.3 aom:3.15.2; do
		name="${spec%%:*}"
		version="${spec#*:}"
		printf 'Name: %s\nDescription: version fixture\nVersion: %s\nLibs:\n' "$name" "$version" \
			>"$XBERG_HEIF_PREFIX/lib/pkgconfig/$name.pc"
	done
	touch "$XBERG_HEIF_PREFIX/lib/libde265.dylib" "$XBERG_HEIF_PREFIX/lib/libaom.dylib"
	xberg_stub lipo '[ "$1" = -archs ] || exit 97' 'printf "x86_64\n"'
	xberg_stub otool 'if [ "$1" = -arch ] && [ "$2" = x86_64 ] && [ "$3" = -l ]; then printf "Load command 0\n cmd LC_BUILD_VERSION\n platform 1\n minos 15.0\n"; else printf "fixture:\nidentity:\n"; fi'
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

@test "macos_heif rejects an unknown profile before downloading" {
	run env XBERG_HEIF_PROFILE=encdoers "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "unknown XBERG_HEIF_PROFILE"
	xberg_assert_trace_empty
}

@test "macos_heif encoder profile requires arm64 at the 15.0 floor" {
	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "requires arm64 and MACOSX_DEPLOYMENT_TARGET=15.0"
	xberg_assert_trace_empty
	xberg_stub uname 'printf "arm64\n"'
	run env XBERG_HEIF_PROFILE=elixir-encoders MACOSX_DEPLOYMENT_TARGET=11.0 "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "requires arm64 and MACOSX_DEPLOYMENT_TARGET=15.0"
	xberg_assert_trace_empty
}

@test "macos_heif encoder profile rejects a populated prefix from another profile" {
	xberg_stub uname 'printf "arm64\n"'
	touch "$XBERG_HEIF_PREFIX/.hidden"
	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "requires a fresh prefix"
	xberg_assert_trace_empty
}

@test "macos_heif encoder profile builds both encoders and stages pinned notices" {
	xberg_stub uname 'printf "arm64\n"'
	xberg_stub lipo 'printf "arm64\n"'
	xberg_stub otool '
		if [ "$1" = -arch ]; then
			printf "Load command 0\n cmd LC_BUILD_VERSION\n platform 1\n minos 15.0\n"
			exit
		fi
		printf "%s:\n" "$2"
		if [ "$1" = -D ]; then
			[ "${XBERG_TEST_SKIP_ID:-}" != "${2##*/}" ] && printf "%s\n" "$2"
			exit 0
		fi
		printf " %s (compatibility version 1.0.0)\n" "$2"
		[ "${XBERG_TEST_SKIP_IMPORTS:-}" = "${2##*/}" ] && exit 0
		printf " /usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n"
		if [[ "$2" == */libheif*.dylib ]]; then
			for dep in libaom libde265 libx265; do
				[ "$dep" = "${XBERG_TEST_SKIP_LINK:-}" ] && continue
				printf " %s/lib/%s.dylib (compatibility version 1.0.0)\n" "$XBERG_HEIF_PREFIX" "$dep"
			done
		fi'
	xberg_stub curl 'printf "%s\n" "${!#}" >>"$XBERG_TRACE"'
	export XBERG_TEST_REAL_SHASUM="$(command -v shasum)"
	xberg_stub shasum '[ "$*" = "-a 256 -c -" ] && exit 0' 'exec "$XBERG_TEST_REAL_SHASUM" "$@"'
	xberg_stub tar '
		case "$2" in
			*/x265.tar.gz) dir=x265_4.3; names="COPYING";;
			*/libde265.tar.gz) dir=libde265-1.1.3; names="COPYING AUTHORS libde265/COPYING";;
			*/libaom.tar.gz) dir=libaom-3.15.3; names="LICENSE PATENTS AUTHORS third_party/fastfeat/LICENSE third_party/libyuv/LICENSE third_party/vector/LICENSE";;
			*/libheif.tar.gz) dir=libheif-1.23.7; names="COPYING";;
			*) exit 93;;
		esac
		mkdir -p "$4/$dir"
		for name in $names; do
			mkdir -p "$(dirname "$4/$dir/$name")"
			printf "%s/%s\n" "$dir" "$name" >"$4/$dir/$name"
		done'
	xberg_stub cmake '
		printf "cmake %s\n" "$*" >>"$XBERG_TRACE"
		[ "$1" = --install ] || exit 0
		case "$2" in
			*/x265-build) name=x265; lib=libx265; version=4.3;;
			*/libde265-build) name=libde265; lib=libde265; version=1.1.3;;
			*/libaom-build) name=aom; lib=libaom; version=3.15.3;;
			*/libheif-build) name=libheif; lib=libheif; version=1.23.7;;
			*) exit 94;;
		esac
		mkdir -p "$XBERG_HEIF_PREFIX/lib/pkgconfig"
		touch "$XBERG_HEIF_PREFIX/lib/$lib.dylib"
		printf "Version: %s\n" "$version" >"$XBERG_HEIF_PREFIX/lib/pkgconfig/$name.pc"'
	export XBERG_HEIF_PREFIX="$XBERG_WORK/fresh-prefix"
	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 0
	grep -q -- 'x265-10 .* -DMAIN12=OFF' "$XBERG_TRACE"
	grep -q -- 'x265-12 .* -DMAIN12=ON' "$XBERG_TRACE"
	grep -q -- '-DLINKED_10BIT=ON -DLINKED_12BIT=ON' "$XBERG_TRACE"
	grep -q -- '-DCONFIG_AV1_ENCODER=1' "$XBERG_TRACE"
	grep -q -- '-DWITH_AOM_ENCODER=ON -DWITH_X265=ON' "$XBERG_TRACE"
	grep -q -- '-DCMAKE_OSX_ARCHITECTURES=arm64' "$XBERG_TRACE"
	local inventory="$XBERG_HEIF_PREFIX/share/xberg-heif-notices"
	[ "$(find "$inventory" -type f | wc -l | tr -d ' ')" = 13 ]
	[ "$(wc -l <"$inventory/sources.tsv" | tr -d ' ')" = 4 ]
	grep -q '83c53e4c8bbb8f1e33ed59e10a7d621d1d7801ca853910c3eb41f038b8ffb121' "$inventory/sources.tsv"
	[ "$(cat "$inventory/libaom-3.15.3-PATENTS")" = 'libaom-3.15.3/PATENTS' ]
	[ "$(cat "$inventory/libaom-3.15.3-third_party-vector-LICENSE")" = 'libaom-3.15.3/third_party/vector/LICENSE' ]

	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains 'already present'

	(cd "$inventory" && shasum -a 256 -c notices.sha256)
	[ "$(wc -l <"$inventory/notices.sha256" | tr -d ' ')" = 12 ]

	run env XBERG_HEIF_PROFILE=elixir-encoders XBERG_TEST_SKIP_ID=libaom.dylib "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing or invalid dylib identity'

	run env XBERG_HEIF_PROFILE=elixir-encoders XBERG_TEST_SKIP_IMPORTS=libaom.dylib "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing or invalid dylib import rows'

	run env XBERG_HEIF_PROFILE=elixir-encoders XBERG_TEST_SKIP_LINK=libx265 "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'libheif does not link to the pinned libx265'

	run "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'decode profile requires a prefix separate from elixir-encoders'

	printf 'modified\n' >"$inventory/x265_4.3-COPYING"
	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing or mismatched codec notice checksums'

	rm "$inventory/x265_4.3-COPYING"
	run env XBERG_HEIF_PROFILE=elixir-encoders "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing upstream codec notice x265_4.3-COPYING'
}

@test "macos_heif static profile requires arm64 at the 15.0 floor and an explicit prefix" {
	run env XBERG_HEIF_PROFILE=php-static "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "php-static requires arm64 and MACOSX_DEPLOYMENT_TARGET=15.0"
	xberg_stub uname 'printf "arm64\n"'
	run env -u XBERG_HEIF_PREFIX XBERG_HEIF_PROFILE=php-static "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "php-static requires an explicit XBERG_HEIF_PREFIX"
	xberg_assert_trace_empty
}

@test "macos_heif static profile builds decode-only archives without x265 and stages three notices" {
	xberg_stub uname 'printf "arm64\n"'
	xberg_stub otool 'if [ "$1" = -arch ]; then printf "Load command 0\n cmd LC_BUILD_VERSION\n platform 1\n minos %s\n" "${XBERG_TEST_MINOS:-15.0}"; fi'
	xberg_stub curl 'printf "%s\n" "${!#}" >>"$XBERG_TRACE"'
	export XBERG_TEST_REAL_SHASUM="$(command -v shasum)"
	xberg_stub shasum '[ "$*" = "-a 256 -c -" ] && exit 0' 'exec "$XBERG_TEST_REAL_SHASUM" "$@"'
	xberg_stub tar '
		case "$2" in
			*/libde265.tar.gz) dir=libde265-1.1.3; names="COPYING AUTHORS libde265/COPYING";;
			*/libaom.tar.gz) dir=libaom-3.15.2; names="LICENSE PATENTS AUTHORS third_party/fastfeat/LICENSE third_party/libyuv/LICENSE third_party/vector/LICENSE";;
			*/libheif.tar.gz) dir=libheif-1.23.7; names="COPYING";;
			*) exit 93;;
		esac
		mkdir -p "$4/$dir"
		for name in $names; do
			mkdir -p "$(dirname "$4/$dir/$name")"
			printf "%s/%s\n" "$dir" "$name" >"$4/$dir/$name"
		done'
	xberg_stub cmake '
		printf "cmake %s\n" "$*" >>"$XBERG_TRACE"
		[ "$1" = --install ] || exit 0
		case "$2" in
			*/libde265-build) name=libde265; lib=libde265; version=1.1.3;;
			*/libaom-build) name=aom; lib=libaom; version=3.15.2;;
			*/libheif-build) name=libheif; lib=libheif; version=1.23.7;;
			*) exit 94;;
		esac
		mkdir -p "$XBERG_HEIF_PREFIX/lib/pkgconfig"
		touch "$XBERG_HEIF_PREFIX/lib/$lib.a"
		[ -z "${XBERG_TEST_DYLIB:-}" ] || touch "$XBERG_HEIF_PREFIX/lib/$lib.dylib"
		printf "Version: %s\n" "$version" >"$XBERG_HEIF_PREFIX/lib/pkgconfig/$name.pc"'
	export XBERG_HEIF_PREFIX="$XBERG_WORK/static-prefix"
	run env XBERG_HEIF_PROFILE=php-static "$BASH" "$SCRIPT"
	xberg_assert_status 0
	grep -q -- '-DBUILD_SHARED_LIBS=OFF' "$XBERG_TRACE"
	! grep -q -- '-DBUILD_SHARED_LIBS=ON' "$XBERG_TRACE"
	grep -q -- '-DWITH_AOM_ENCODER=OFF -DWITH_X265=OFF' "$XBERG_TRACE"
	grep -q -- '-DCONFIG_AV1_ENCODER=0' "$XBERG_TRACE"
	grep -q -- '-DENABLE_PLUGIN_LOADING=OFF' "$XBERG_TRACE"
	[ "$(grep -c -i x265 "$XBERG_TRACE")" = 1 ]
	local inventory="$XBERG_HEIF_PREFIX/share/xberg-heif-notices"
	[ "$(wc -l <"$inventory/sources.tsv" | tr -d ' ')" = 3 ]
	! grep -q x265 "$inventory/sources.tsv"
	[ "$(find "$inventory" -type f | wc -l | tr -d ' ')" = 13 ]
	cmp "$inventory/GPL-3.0.txt" "$REPO_ROOT/scripts/ci/notices/GPL-3.0.txt"
	grep -q '^[0-9a-f]\{64\}  GPL-3.0.txt$' "$inventory/notices.sha256"
	(cd "$inventory" && shasum -a 256 -c notices.sha256)

	run env XBERG_HEIF_PROFILE=php-static "$BASH" "$SCRIPT"
	xberg_assert_status 0
	xberg_assert_output_contains 'already present'

	printf 'altered\n' >"$inventory/GPL-3.0.txt"
	run env XBERG_HEIF_PROFILE=php-static "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing or altered GPL-3.0.txt'
	cp "$REPO_ROOT/scripts/ci/notices/GPL-3.0.txt" "$inventory/GPL-3.0.txt"

	rm -rf "$XBERG_HEIF_PREFIX"
	run env XBERG_HEIF_PROFILE=php-static XBERG_TEST_MINOS=26.0 "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "objects declare macOS '26.0', expected only 15.0"

	rm -rf "$XBERG_HEIF_PREFIX"
	run env XBERG_HEIF_PROFILE=php-static XBERG_TEST_DYLIB=1 "$BASH" "$SCRIPT"
	xberg_assert_status 1
	xberg_assert_output_contains "static prefix contains"
}
