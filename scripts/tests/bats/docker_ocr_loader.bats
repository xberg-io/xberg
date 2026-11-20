#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
}

# The runtime stage installs the distro tesseract-ocr package, which brings its own
# libleptonica.so.6. ldconfig keeps the first directory that holds a soname, and
# aarch64-linux-gnu.conf sorts before libc.conf, so /usr/local/lib must be listed
# in a file that sorts before every distro entry, and before the first ldconfig
# that feeds the OCR runtime check.
assert_local_lib_first() {
	local dockerfile="$1" conf_line check_line
	conf_line="$(grep -n 'ld\.so\.conf\.d/00-[^ ]*\.conf' "$dockerfile" | head -n1 | cut -d: -f1)"
	check_line="$(grep -n 'verify-ocr-linux\.py /usr/local' "$dockerfile" | head -n1 | cut -d: -f1)"
	[ -n "$conf_line" ] && [ -n "$check_line" ] || return 1
	[ "$conf_line" -le "$check_line" ] || return 1
	grep -Fq 'echo /usr/local/lib >/etc/ld.so.conf.d/00-' "$dockerfile"
}

@test "core Dockerfile should rank /usr/local/lib before distro libraries" {
	assert_local_lib_first "$REPOSITORY_ROOT/docker/Dockerfile.core"
}

@test "full Dockerfile should rank /usr/local/lib before distro libraries" {
	assert_local_lib_first "$REPOSITORY_ROOT/docker/Dockerfile.full"
}

@test "loader contract should reject a Dockerfile without the early ld.so entry" {
	sed 's#echo /usr/local/lib >/etc/ld.so.conf.d/00-xberg-ocr.conf && ##' "$REPOSITORY_ROOT/docker/Dockerfile.core" >"$XBERG_WORK/Dockerfile"
	run assert_local_lib_first "$XBERG_WORK/Dockerfile"
	xberg_assert_status 1
}
