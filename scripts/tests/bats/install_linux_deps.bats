setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	INSTALL_SCRIPT="$REPO_ROOT/scripts/ci/install-system-deps/install-linux.sh"
	export LIBHEIF_PREFIX="$XBERG_WORK/libheif" LIBHEIF_VERSION=1.19.8
	export PKG_CONFIG_LIBDIR="$LIBHEIF_PREFIX/lib/pkgconfig" PKG_CONFIG_PATH=
	mkdir -p "$PKG_CONFIG_LIBDIR"
	cat >"$PKG_CONFIG_LIBDIR/libheif.pc" <<'PC'
Name: libheif
Description: cached libheif requiring an external development package
Version: 1.19.8
Requires.private: libsharpyuv
Libs: -lheif
Cflags: -I/example/heif
PC
	awk '/^libheif_search_path\(\)/{capture=1} /^LIBHEIF_CACHE=/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/libheif-checks.sh"
	grep -q '^libheif_candidate_is_usable()' "$XBERG_WORK/libheif-checks.sh"
	mkdir -p "$LIBHEIF_PREFIX/include/libheif"
	printf '%s\n' '/* header */' >"$LIBHEIF_PREFIX/include/libheif/heif.h"
	touch "$LIBHEIF_PREFIX/lib/libheif.so.1"
	ln -s libheif.so.1 "$LIBHEIF_PREFIX/lib/libheif.so"
}

@test "install_linux should_reject_cached_libheif_when_required_pkg_config_dependency_is_missing" {
	run pkg-config --modversion libheif
	xberg_assert_status 0
	xberg_assert_output "1.19.8"
	run pkg-config --cflags --libs libheif
	xberg_assert_status 1
	xberg_assert_output_contains "libsharpyuv"

	run bash -euo pipefail -c 'source "$1"; libheif_candidate_is_usable "$LIBHEIF_PREFIX"' \
		_ "$XBERG_WORK/libheif-checks.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "libsharpyuv"
}

@test "install_linux should_accept_cached_libheif_when_required_pkg_config_dependency_is_available" {
	cat >"$PKG_CONFIG_LIBDIR/libsharpyuv.pc" <<'PC'
Name: libsharpyuv
Description: complete dependency fixture
Version: 0.2.0
Libs: -lsharpyuv
Cflags: -I/example/sharpyuv
PC
	run pkg-config --cflags --libs libheif
	xberg_assert_status 0
	local flags
	read -r -a flags <<<"$output"
	[ "${flags[*]}" = "-I/example/heif -I/example/sharpyuv -lheif" ]


}

@test "install_linux should_install_sharpyuv_development_metadata_for_cached_libheif" {
	awk '/^packages=\(/{capture=1} /^echo "Installing dependencies/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/packages.sh"
	[ "$(grep -c '^packages=(' "$XBERG_WORK/packages.sh")" -eq 1 ]
	xberg_stub_exit apt-cache 0
	run bash -euo pipefail -c 'source "$1"; printf "%s\n" "${packages[@]}" | grep -cx libsharpyuv-dev' \
		_ "$XBERG_WORK/packages.sh"
	xberg_assert_status 0
	xberg_assert_output "1"
}

@test "install_linux should_preserve_older_distribution_packages_when_sharpyuv_development_package_is_unavailable" {
	awk '/^packages=\(/{capture=1} /^echo "Installing dependencies/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/packages.sh"
	[ "$(grep -c '^packages=(' "$XBERG_WORK/packages.sh")" -eq 1 ]
	xberg_stub_exit apt-cache 1
	run bash -euo pipefail -c 'source "$1"; printf "%s\n" "${packages[@]}" | grep -cx libsharpyuv-dev' \
		_ "$XBERG_WORK/packages.sh"
	xberg_assert_status 1
	xberg_assert_output "0"
}

@test "install_linux should_default_to_the_current_security_release" {
	awk '/^LIBHEIF_VERSION=/{capture=1} /^LIBHEIF_PREFIX=/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/select-version.sh"
	[ "$(grep -c '^LIBHEIF_VERSION=' "$XBERG_WORK/select-version.sh")" -eq 1 ]
	run env -u LIBHEIF_VERSION bash -euo pipefail -c 'source "$1"; printf "%s\n" "$LIBHEIF_VERSION"' \
		_ "$XBERG_WORK/select-version.sh"
	xberg_assert_status 0
	xberg_assert_output 1.23.7
}

@test "install_linux should_allow_an_explicit_minimum_abi_runtime" {
	awk '/^LIBHEIF_VERSION=/{capture=1} /^LIBHEIF_PREFIX=/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/select-version.sh"
	[ "$(grep -c '^LIBHEIF_VERSION=' "$XBERG_WORK/select-version.sh")" -eq 1 ]
	run bash -euo pipefail -c 'source "$1"; printf "%s\n" "$LIBHEIF_VERSION"' _ "$XBERG_WORK/select-version.sh"
	xberg_assert_status 0
	xberg_assert_output 1.19.8
}

@test "install_linux should_reject_an_unverified_runtime_version" {
	awk '/^LIBHEIF_VERSION=/{capture=1} /^LIBHEIF_PREFIX=/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/select-version.sh"
	[ "$(grep -c '^LIBHEIF_VERSION=' "$XBERG_WORK/select-version.sh")" -eq 1 ]
	run env LIBHEIF_VERSION=9.9.9 bash -euo pipefail "$XBERG_WORK/select-version.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "Unverified libheif version: 9.9.9"
}

prepare_cache_candidate() {
	export GITHUB_ACTION=install-linux
	CACHE_CANDIDATE="$XBERG_WORK/cache"
	mkdir -p "$CACHE_CANDIDATE/lib/pkgconfig" "$CACHE_CANDIDATE/include/libheif"
	printf '%s\n' '/* header */' >"$CACHE_CANDIDATE/include/libheif/heif.h"
	printf '%s\n' 'ELF fixture' >"$CACHE_CANDIDATE/lib/libheif.so.1"
	ln -s libheif.so.1 "$CACHE_CANDIDATE/lib/libheif.so"
	sed '/^Requires.private:/d' "$PKG_CONFIG_LIBDIR/libheif.pc" >"$CACHE_CANDIDATE/lib/pkgconfig/libheif.pc"
	xberg_stub readelf 'echo " Type: DYN (Shared object file)"' 'echo " (SONAME) Library soname: [libheif.so.1]"'
	xberg_stub ldd 'echo "libx265.so.199 => /usr/lib/libx265.so.199 (0x123)"'
	awk -v cache="$CACHE_CANDIDATE" '
		/^LIBHEIF_CACHE=/{capture=1; printf "LIBHEIF_CACHE=\042%s\042\n", cache; next}
		/^  echo "Building libheif/{capture=0}
		capture' "$INSTALL_SCRIPT" >"$XBERG_WORK/check-cache.sh"
	printf '%s\n' '  echo rebuilding' 'fi' >>"$XBERG_WORK/check-cache.sh"
}

run_cache_check() {
	run bash -euo pipefail -c 'source "$1"; source "$2"' \
		_ "$XBERG_WORK/libheif-checks.sh" "$XBERG_WORK/check-cache.sh"
	xberg_assert_status 0
}

@test "install_linux should rebuild a cached libheif with an unresolved x265 SONAME" {
	prepare_cache_candidate
	xberg_stub ldd 'echo "libx265.so.215 => not found"'
	run_cache_check
	xberg_assert_output_contains "libx265.so.215 => not found"
	xberg_assert_output_contains "rebuilding"
}

@test "install_linux should reuse staged libheif with positive linkage and no empty search entries" {
	prepare_cache_candidate
	export LD_LIBRARY_PATH=':/opt/codecs::;/opt/extra:'
	xberg_stub ldd '[[ "$1" == -r ]] || exit 2' \
		'[[ "$2" == "$CACHE_CANDIDATE/lib/libheif.so.1" ]] || exit 3' \
		'[[ "$LD_LIBRARY_PATH" == "$CACHE_CANDIDATE/lib:/opt/codecs:/opt/extra" ]] || exit 4' \
		'echo "libx265.so.199 => /usr/lib/libx265.so.199 (0x123)"'
	export CACHE_CANDIDATE
	run_cache_check
	xberg_assert_output_contains "Reusing cached libheif"
	[[ "$output" != *rebuilding* ]]
}

@test "install_linux should rebuild an incomplete exact cache hit" {
	prepare_cache_candidate
	mv "$CACHE_CANDIDATE/include/libheif/heif.h" "$XBERG_WORK/missing-header"
	xberg_stub ldd 'echo "unexpected ldd invocation"; exit 2'
	run_cache_check
	xberg_assert_output_contains "Incomplete libheif candidate"
	xberg_assert_output_contains "rebuilding"
	[[ "$output" != *"unexpected ldd invocation"* ]]
	[ ! -s "$LIBHEIF_PREFIX/lib/libheif.so.1" ]
}

@test "install_linux should reject non-ELF even when ldd claims a dependency resolved" {
	prepare_cache_candidate
	xberg_stub readelf 'echo "not an ELF file" >&2; exit 1'
	run_cache_check
	xberg_assert_output_contains "Expected a libheif.so.1 ELF shared library"
	xberg_assert_output_contains "rebuilding"
}

@test "install_linux should reject a successful loader probe without a resolved dependency" {
	prepare_cache_candidate
	xberg_stub ldd 'echo "statically linked"'
	run_cache_check
	xberg_assert_output_contains "Cannot verify runtime dependencies"
	xberg_assert_output_contains "rebuilding"
}

@test "install_linux should replace stale cache contents with the freshly staged tree" {
	prepare_cache_candidate
	touch "$CACHE_CANDIDATE/lib/libheif.so.stale"
	export LIBHEIF_CACHE="$CACHE_CANDIDATE"
	export candidate="$XBERG_WORK/new"
	mkdir -p "$candidate/lib"
	printf '%s\n' fresh >"$candidate/lib/libheif.so.1"
	awk '/^if \[\[ -n .*GITHUB_ACTION.*&& "\$candidate" != "\$LIBHEIF_CACHE"/{capture=1} capture; capture && /^fi$/{exit}' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/seed-cache.sh"
	grep -q 'mv "\$candidate"' "$XBERG_WORK/seed-cache.sh"
	run bash -euo pipefail "$XBERG_WORK/seed-cache.sh"
	xberg_assert_status 0
	[ "$(cat "$LIBHEIF_CACHE/lib/libheif.so.1")" = fresh ]
	[ ! -e "$LIBHEIF_CACHE/lib/libheif.so.stale" ]
}

run_cache_promotion() {
	run bash -euo pipefail -c 'source "$1"; promote_libheif "$2" "$LIBHEIF_PREFIX"' \
		_ "$XBERG_WORK/libheif-checks.sh" "$CACHE_CANDIDATE"
}

@test "install_linux should leave the destination untouched when candidate lib is an external symlink" {
	prepare_cache_candidate
	mv "$CACHE_CANDIDATE/lib" "$XBERG_WORK/external-lib"
	ln -s "$XBERG_WORK/external-lib" "$CACHE_CANDIDATE/lib"
	printf '%s\n' original >"$LIBHEIF_PREFIX/lib/libheif.so.1"
	xberg_stub sudo 'printf "%s\n" "$*" >>"$XBERG_TRACE"; "$@"'
	run_cache_check
	xberg_assert_output_contains "rebuilding"
	run_cache_promotion
	xberg_assert_status 1
	[ ! -e "$XBERG_TRACE" ]
	[ "$(cat "$LIBHEIF_PREFIX/lib/libheif.so.1")" = original ]
}


@test "install_linux should reject external header symlinks before privileged promotion" {
	prepare_cache_candidate
	printf '%s\n' original >"$LIBHEIF_PREFIX/include/libheif/heif.h"
	ln -s "$LIBHEIF_PREFIX/include/libheif/heif.h" "$CACHE_CANDIDATE/include/libheif/heif_external.h"
	xberg_stub sudo 'printf "%s\n" "$*" >>"$XBERG_TRACE"; "$@"'
	run_cache_promotion
	xberg_assert_status 1
	[ ! -e "$XBERG_TRACE" ]
	[ "$(cat "$LIBHEIF_PREFIX/include/libheif/heif.h")" = original ]
}

@test "install_linux should pass only expected artifacts to privileged copy" {
	prepare_cache_candidate
	mkdir -p "$CACHE_CANDIDATE/bin"
	touch "$CACHE_CANDIDATE/bin/unexpected" "$CACHE_CANDIDATE/lib/libunrelated.so"
	xberg_stub sudo 'printf "%s\n" "$@" >>"$XBERG_TRACE"'
	run_cache_promotion
	xberg_assert_status 0
	local expected
	expected="$(printf '%s\n' mkdir -p "$LIBHEIF_PREFIX" cp -P --parents --remove-destination -- lib/libheif.so.1 lib/libheif.so \
		lib/pkgconfig/libheif.pc include/libheif/heif.h "$LIBHEIF_PREFIX/")"
	[ "$(cat "$XBERG_TRACE")" = "$expected" ]
}
