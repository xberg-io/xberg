#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p payload ort inspected
	printf 'extension payload\n' >payload/xberg.so
	printf 'license payload\n' >payload/NOTICE
	printf 'runtime payload\n' >ort/libonnxruntime.so.1.24.2
	ln -s libonnxruntime.so.1.24.2 ort/libonnxruntime.so.1
	tar -czf extension.tgz -C payload xberg.so NOTICE
	export DYNAMIC_FIXTURE="$BATS_TEST_TMPDIR/dynamic" SYMBOL_FIXTURE="$BATS_TEST_TMPDIR/symbols"
	export ORT_SYMBOL_FIXTURE="$BATS_TEST_TMPDIR/ort-symbols" OBJDUMP_STATUS=0
	printf '%s\n' '  NEEDED libonnxruntime.so.1' '  RUNPATH $ORIGIN' '  FLAGS_1 0x00000008' >"$DYNAMIC_FIXTURE"
	printf '%s\n' '0000000000 g DF .text 0 GLIBC_2.36 memset' \
		'0000000000 g DF .text 0 GLIBCXX_3.4.30 fixture' \
		'0000000000 g DF .text 0 CXXABI_1.3.13 fixture' >"$SYMBOL_FIXTURE"
	cp "$SYMBOL_FIXTURE" "$ORT_SYMBOL_FIXTURE"
	unset EXPECTED_XBERG_SHA256
	xberg_stub objdump \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "$OBJDUMP_STATUS" = 0 ] || exit "$OBJDUMP_STATUS"' \
		'if [ "$1" = -p ]; then cat "$DYNAMIC_FIXTURE"; elif [[ "$2" = */libonnxruntime.so* ]]; then cat "$ORT_SYMBOL_FIXTURE"; else cat "$SYMBOL_FIXTURE"; fi'
}

@test "PHP staging should dereference the exact needed runtime and regenerate the archive checksum" {
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 0
	tar -xzf extension.tgz -C inspected
	xberg_assert_file inspected/xberg.so 'extension payload'
	xberg_assert_file inspected/libonnxruntime.so.1 'runtime payload'
	xberg_assert_file inspected/NOTICE 'license payload'
	[ ! -L inspected/libonnxruntime.so.1 ]
	[ -x inspected/libonnxruntime.so.1 ]
	expected_hash="$(shasum -a 256 extension.tgz | awk '{print $1}')"
	xberg_assert_file extension.tgz.sha256 "$expected_hash  extension.tgz"
	[ "$(tar -tzf extension.tgz | wc -l | tr -d '[:space:]')" = 3 ]
	xberg_assert_output_contains '3 archive entries'
}

@test "PHP staging should reject a static ORT extension without rewriting the source artifact" {
	printf '%s\n' '  NEEDED libc.so.6' >"$DYNAMIC_FIXTURE"
	before="$(shasum -a 256 extension.tgz)"
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 1
	xberg_assert_output_contains 'does not link libonnxruntime dynamically'
	[ "$(shasum -a 256 extension.tgz)" = "$before" ]
	[ ! -e extension.tgz.sha256 ]
}

@test "PHP staging should reject an extension that cannot resolve a neighboring runtime" {
	printf '%s\n' '  NEEDED libonnxruntime.so.1' >"$DYNAMIC_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 1
	xberg_assert_output_contains 'has no $ORIGIN rpath'
	[ ! -e extension.tgz.sha256 ]
}

@test "PHP staging should reject a missing needed runtime without repacking the archive" {
	rm ort/libonnxruntime.so.1 ort/libonnxruntime.so.1.24.2
	before="$(shasum -a 256 extension.tgz)"
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 1
	xberg_assert_output_contains 'no libonnxruntime.so.1 under ort'
	[ "$(shasum -a 256 extension.tgz)" = "$before" ]
}

@test "PHP staging should require the extension at the archive root" {
	mkdir -p payload/nested
	mv payload/xberg.so payload/nested/xberg.so
	tar -czf extension.tgz -C payload nested
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 1
	xberg_assert_output_contains 'xberg.so is not at the archive root'
	[ ! -e "$XBERG_TRACE" ]
}

@test "PHP staging should reject an unreadable ELF report before rewriting the archive" {
	export OBJDUMP_STATUS=7
	run bash "$REPOSITORY_ROOT/scripts/ci/stage-php-linux-ort.sh" extension.tgz ort
	xberg_assert_status 1
	xberg_assert_output_contains 'is not a readable ELF shared library'
	[ ! -e extension.tgz.sha256 ]
}

stage_abi_archive() {
	cp ort/libonnxruntime.so.1.24.2 payload/libonnxruntime.so.1
	tar -czf extension.tgz -C payload xberg.so libonnxruntime.so.1 NOTICE
}

@test "PHP ABI gate should inspect both libraries and verify the exact packaged extension identity" {
	stage_abi_archive
	export EXPECTED_XBERG_SHA256
	EXPECTED_XBERG_SHA256="$(shasum -a 256 payload/xberg.so | awk '{print $1}')"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 0
	xberg_assert_output_contains 'packaged xberg.so matches the floor-built artifact'
	xberg_assert_output_contains 'artifact satisfies the Debian 12 ABI floor'
	[ "$(grep -c '^-T ' "$XBERG_TRACE")" = 2 ]
	grep -Eq '^-T .*/xberg\.so$' "$XBERG_TRACE"
	grep -Eq '^-T .*/libonnxruntime\.so\.1$' "$XBERG_TRACE"
}

@test "PHP ABI gate should reject no GLIBC requirements instead of accepting zero examined symbols" {
	stage_abi_archive
	printf 'no versioned symbols\n' >"$SYMBOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'ABI gate examined no glibc requirements'
}

@test "PHP ABI gate should reject a runtime that exceeds the selected glibc floor" {
	stage_abi_archive
	printf '0000 g DF .text 0 GLIBC_2.37 fixture\n' >"$ORT_SYMBOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'libonnxruntime.so.1 requires GLIBC 2.37 > 2.36'
}

@test "PHP ABI gate should reject an extension that exceeds the selected C++ ABI floors" {
	stage_abi_archive
	printf '%s\n' '0000 g DF .text 0 GLIBC_2.36 memset' \
		'0000 g DF .text 0 GLIBCXX_3.4.31 fixture' \
		'0000 g DF .text 0 CXXABI_1.3.14 fixture' >"$SYMBOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'requires GLIBCXX 3.4.31 > 3.4.30'
	xberg_assert_output_contains 'requires CXXABI 1.3.14 > 1.3.13'
}

@test "PHP ABI gate should reject unresolved unversioned C++ symbols" {
	stage_abi_archive
	printf '0000 g *UND* 0 Base _Zfixture\n' >>"$SYMBOL_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'leaves 1 C++ symbols undefined and unversioned'
}

@test "PHP ABI gate should reject duplicate extensions before examining a dynamic library" {
	stage_abi_archive
	mkdir -p payload/nested
	cp payload/xberg.so payload/nested/xberg.so
	tar -czf extension.tgz -C payload xberg.so libonnxruntime.so.1 nested
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'expected exactly one xberg.so, found 2'
	[ ! -e "$XBERG_TRACE" ]
}

@test "PHP ABI gate should reject an extension hash mismatch before running objdump" {
	stage_abi_archive
	export EXPECTED_XBERG_SHA256=0000000000000000000000000000000000000000000000000000000000000000
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'does not match the floor-built artifact'
	[ ! -e "$XBERG_TRACE" ]
}

@test "PHP ABI gate should require NODELETE and ORIGIN loader protections" {
	stage_abi_archive
	printf '%s\n' '  NEEDED libonnxruntime.so.1' >"$DYNAMIC_FIXTURE"
	run bash "$REPOSITORY_ROOT/scripts/ci/verify-php-linux-abi.sh" extension.tgz
	xberg_assert_status 1
	xberg_assert_output_contains 'has no $ORIGIN rpath'
	xberg_assert_output_contains 'is not NODELETE'
}
