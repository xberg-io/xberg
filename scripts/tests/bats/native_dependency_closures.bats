#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	XBERG_WORK="$(pwd -P)"
	export XBERG_WORK
	export SOURCE_DIR="$XBERG_WORK/source" STATE_DIR="$XBERG_WORK/state" REAL_PYTHON
	REAL_PYTHON="$(command -v python3)"
	export REAL_MKTEMP
	REAL_MKTEMP="$(command -v mktemp)"
	xberg_stub mktemp 'path="$("$REAL_MKTEMP" "$@")"' 'printf "%s/%s\n" "$(cd "${path%/*}" && pwd -P)" "${path##*/}"'
	mkdir source state bundle
	unset OTOOL_FAIL_CALL REWRITE_STATUS SIGN_STATUS MACOSX_DEPLOYMENT_TARGET LDD_MISSING LDD_EXTERNAL LDD_STATUS DLOPEN_STATUS PATCH_STATUS
	stub_macho_tools
	stub_elf_tools
}

stub_macho_tools() {
	xberg_stub otool \
		'printf "otool %s\n" "$*" >>"$XBERG_TRACE"' \
		'file="${!#}"; name="${file##*/}"; id="$file"; [ ! -f "$STATE_DIR/$name.id" ] || id="$(cat "$STATE_DIR/$name.id")"' \
		'case "$1" in' \
		'-D) printf "%s:\n%s\n" "$file" "$id" ;;' \
		'-l) printf "LC_BUILD_VERSION\nminos 12.0\n" ;;' \
		'-L) count=0; [ ! -f "$STATE_DIR/otool-count" ] || count="$(cat "$STATE_DIR/otool-count")"; count=$((count+1)); printf "%s\n" "$count" >"$STATE_DIR/otool-count"' \
		'    [ "$count" != "${OTOOL_FAIL_CALL:-0}" ] || { echo "load-command inspection failed" >&2; exit 9; }' \
		'    printf "%s:\n\t%s (compatibility version 1.0.0)\n" "$file" "$id"' \
		'    while IFS= read -r dependency; do printf "\t%s (compatibility version 1.0.0)\n" "$dependency"; done <"$STATE_DIR/$name.deps" ;;' \
		'*) exit 97 ;; esac'
	xberg_stub install_name_tool \
		'printf "rewrite %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${REWRITE_STATUS:-0}" = 0 ] || exit "$REWRITE_STATUS"' \
		'if [ "$1" = -id ]; then printf "%s\n" "$2" >"$STATE_DIR/${3##*/}.id"; exit 0; fi' \
		'"$REAL_PYTHON" -c '\''import pathlib,sys; p=pathlib.Path(sys.argv[1]); p.write_text(p.read_text().replace(sys.argv[2]+"\n",sys.argv[3]+"\n"))'\'' "$STATE_DIR/${4##*/}.deps" "$2" "$3"'
	xberg_stub codesign \
		'printf "codesign %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1" in --verify) exit "${SIGN_STATUS:-0}" ;; --remove-signature) exit 1 ;; *) exit 0 ;; esac'
}

stub_elf_tools() {
	xberg_stub patchelf 'printf "patchelf %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${PATCH_STATUS:-0}"'
	xberg_stub readlink '"$REAL_PYTHON" -c '\''import os,sys; print(os.path.realpath(sys.argv[1]))'\'' "$2"'
	xberg_stub readelf 'printf " (NEEDED) Shared library: [libcodec.so]\n"'
	xberg_stub python3 \
		'printf "dlopen %s\n" "${!#}" >>"$XBERG_TRACE"' \
		'case "$2" in *ctypes.CDLL*) ;; *) exit 97 ;; esac' \
		'[ "${DLOPEN_STATUS:-0}" = 0 ] || { echo "loader rejected artifact" >&2; exit "$DLOPEN_STATUS"; }'
	xberg_stub ldd \
		'printf "ldd %s\n" "$*" >>"$XBERG_TRACE"' \
		'file="$1"; directory="${file%/*}"; name="${file##*/}"' \
		'case "$name" in libxberg_ffi.so) dependency=libcodec.so ;; libcodec.so) dependency=libsupport.so ;; *) dependency="" ;; esac' \
		'if [ -n "$dependency" ]; then if [ -f "$directory/$dependency" ] && [ "${LDD_EXTERNAL:-false}" != true ]; then resolved="$directory/$dependency"; else resolved="$SOURCE_DIR/$dependency"; fi; printf "\t%s => %s (0x123)\n" "$dependency" "$resolved"; fi' \
		'if [ "${LDD_MISSING:-false}" = true ]; then printf "libmissing.so => not found\n"; fi' \
		'if [ "${LDD_STATUS:-0}" != 0 ]; then printf "Error relocating %s: PyExample: symbol not found\n" "$file"; fi' \
		'exit "${LDD_STATUS:-0}"'
}

prepare_macho() {
	local name="$1"
	printf 'root native bytes\n' >"bundle/$name"
	printf 'codec native bytes\n' >source/libcodec.dylib
	printf 'support native bytes\n' >source/libsupport.dylib
	printf '%s\n' "$SOURCE_DIR/libcodec.dylib" /usr/lib/libSystem.B.dylib >"state/$name.deps"
	printf '%s\n' "$SOURCE_DIR/libsupport.dylib" >state/libcodec.dylib.deps
	printf '%s\n' /usr/lib/libSystem.B.dylib >state/libsupport.dylib.deps
}

prepare_elf() {
	printf 'root ELF bytes\n' >bundle/libxberg_ffi.so
	printf 'codec ELF bytes\n' >source/libcodec.so
	printf 'support ELF bytes\n' >source/libsupport.so
}

assert_macho_bundle() {
	local expected_changes="${1:-2}"
	xberg_assert_file bundle/libcodec.dylib 'codec native bytes'
	xberg_assert_file bundle/libsupport.dylib 'support native bytes'
	[ "$(find bundle -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
	[ "$(grep -c '^otool -L ' "$XBERG_TRACE")" -eq 6 ]
	[ "$(grep -c '^codesign --verify --strict ' "$XBERG_TRACE")" -eq 3 ]
	[ "$(grep -c '^rewrite -change ' "$XBERG_TRACE")" -eq "$expected_changes" ]
}

@test "macOS native closure should vendor transitive bytes once and verify every relocated signature" {
	prepare_macho app
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 0
	assert_macho_bundle
	grep -Fxq '@loader_path/libcodec.dylib' state/app.deps
	grep -Fxq '@loader_path/libsupport.dylib' state/libcodec.dylib.deps
}

@test "macOS native closure should reject a referenced dependency missing on disk" {
	prepare_macho app
	rm source/libcodec.dylib
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 1
	xberg_assert_output_contains 'dependency not found on disk'
}

@test "macOS native closure should reject failed initial load-command inspection" {
	prepare_macho app
	export OTOOL_FAIL_CALL=1
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to inspect dependencies'
}

@test "macOS native closure should reject failed final inspection after successful vendoring" {
	prepare_macho app
	export OTOOL_FAIL_CALL=4
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to inspect final closure'
}

@test "macOS native closure should reject invalid signatures even when re-signing returned success" {
	prepare_macho app
	export SIGN_STATUS=7
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 1
	xberg_assert_output_contains 'no valid code signature'
}

@test "macOS native closure should preserve load-command rewrite failures" {
	prepare_macho app
	export REWRITE_STATUS=9
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure-macos.sh" "$XBERG_WORK/bundle/app"
	xberg_assert_status 1
	xberg_assert_output_contains 'install_name_tool -change failed'
}

@test "macOS Node closure should vendor transitive dylibs and verify each changed artifact" {
	prepare_macho addon.node
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 0
	assert_macho_bundle
	grep -Fxq '@loader_path/libcodec.dylib' state/addon.node.deps
}

@test "macOS Node closure should visit each artifact once when dylibs have cyclic dependencies" {
	prepare_macho addon.node
	printf '%s\n' "$SOURCE_DIR/libcodec.dylib" >state/libsupport.dylib.deps
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 0
	assert_macho_bundle 3
}

@test "macOS Node closure should reject an empty directory instead of verifying zero native files" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'no .node'
}

@test "macOS Node closure should reject failed dependency inspection instead of declaring self-containment" {
	prepare_macho addon.node
	export OTOOL_FAIL_CALL=1
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to inspect dependencies'
}

@test "macOS Node closure should reject failed final inspection after relocating dependencies" {
	prepare_macho addon.node
	export OTOOL_FAIL_CALL=4
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to inspect final closure'
}

@test "macOS Node closure should reject Homebrew source libraries before copying them" {
	prepare_macho addon.node
	printf '%s\n' /opt/homebrew/lib/libcodec.dylib >state/addon.node.deps
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'refusing to vendor Homebrew dylib'
	[ ! -e bundle/libcodec.dylib ]
}

@test "macOS Node closure should reject a package whose node artifact exceeds its deployment floor" {
	prepare_macho addon.node
	export MACOSX_DEPLOYMENT_TARGET=11.0
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-macos-node-dylibs.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'targets macOS 12.0, expected 11.0'
}

@test "ELF closure should copy transitive library bytes and set ORIGIN on every bundled artifact" {
	prepare_elf
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 0
	xberg_assert_file bundle/libcodec.so 'codec ELF bytes'
	xberg_assert_file bundle/libsupport.so 'support ELF bytes'
	[ "$(find bundle -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
	[ "$(grep -c '^ldd ' "$XBERG_TRACE")" -eq 4 ]
	[ "$(grep -c '^patchelf --set-rpath \$ORIGIN ' "$XBERG_TRACE")" -eq 3 ]
	[ "$(grep -c '^dlopen ' "$XBERG_TRACE")" -eq 1 ]
}

@test "ELF closure should reject unresolved shared libraries even if ldd exits zero" {
	prepare_elf
	export LDD_MISSING=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 1
	xberg_assert_output_contains 'missing a shared library after vendoring'
}

@test "ELF closure should reject dependencies that still resolve outside the bundle" {
	prepare_elf
	export LDD_EXTERNAL=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 1
	xberg_assert_output_contains 'still resolves outside the bundle'
}

@test "ELF closure should allow host-provided Python symbols only after its loader probe succeeds" {
	prepare_elf
	export LDD_STATUS=127
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 0
	xberg_assert_output_contains 'undefined symbols the host process supplies'
	[ "$(grep -c '^dlopen ' "$XBERG_TRACE")" -eq 1 ]
}

@test "ELF closure should reject a loader failure after ldd reports complete dependencies" {
	prepare_elf
	export DLOPEN_STATUS=7
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 1
	xberg_assert_output_contains 'failed to dlopen after vendoring'
}

@test "ELF closure should propagate failed RPATH rewriting before declaring a verified bundle" {
	prepare_elf
	export PATCH_STATUS=9
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle/libxberg_ffi.so"
	xberg_assert_status 9
	run grep -q '^dlopen ' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "ELF closure should reject a directory containing zero supported native artifacts" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/bundle"
	xberg_assert_status 1
	xberg_assert_output_contains 'no xberg native library found'
	xberg_assert_trace_empty
}

@test "ELF closure should repack a real archive with all discovered native dependency bytes" {
	prepare_elf
	tar -czf native.tgz -C bundle .
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/vendor-native-closure.sh" "$XBERG_WORK/native.tgz"
	xberg_assert_status 0
	mkdir extracted
	tar -xzf native.tgz -C extracted
	xberg_assert_file extracted/libxberg_ffi.so 'root ELF bytes'
	xberg_assert_file extracted/libcodec.so 'codec ELF bytes'
	xberg_assert_file extracted/libsupport.so 'support ELF bytes'
	[ "$(find extracted -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
}
